//! Event types and the main event loop driver for the Redwood TUI.
//!
//! This module defines the [`Event`] enum (keyboard input, ticks, flight updates,
//! and DB init messages) and the [`EventHandler`], which runs a background task
//! that polls crossterm for key events and emits periodic [`Event::Tick`]s.
//! The main loop in `main.rs` receives events via [`EventHandler::next`] and
//! other tasks (e.g. the API poller) send events via [`EventHandler::tx`].

use crate::models::Flight;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Events processed by the application event loop.
///
/// The main loop in `main.rs` matches on these to update [`App`](crate::app::App) state
/// and drive the UI (e.g. key handling, tick updates, flight list updates).
pub enum Event {
    /// Periodic tick used for UI refresh and init progress draining.
    Tick,
    /// User key press from the terminal.
    Input(KeyEvent),
    /// New flight data from the API poller (or a failed fetch).
    FlightUpdate {
        /// Flights in the area; may be empty on API failure.
        flights: Vec<Flight>,
        /// Number of flights that were enriched with DB data.
        db_hits: usize,
        /// When this update was produced.
        timestamp: std::time::Instant,
        /// Whether the API request succeeded.
        is_success: bool,
    },
    /// Database initialization progress (0.0 to 1.0).
    DbProgress(f32),
    /// Database initialization completed successfully.
    DbDone,
    /// Database initialization failed; payload is the error message.
    DbError(String),
}

/// Multiplexes terminal input and ticks into a single event stream.
///
/// Holds an unbounded channel: the sender ([`tx`](EventHandler::tx)) can be
/// cloned and given to other tasks (e.g. the API poller), while the receiver
/// is consumed by [`next`](EventHandler::next) in the main loop. A background
/// task polls crossterm with a timeout and sends [`Event::Input`] on key press
/// and [`Event::Tick`] at the configured interval.
pub struct EventHandler {
    /// Sender for posting events (e.g. from the API poller or DB init thread).
    pub tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    /// Creates a new event handler and spawns the input/tick task.
    ///
    /// The spawned task runs until the process exits. It polls crossterm with
    /// a timeout of `tick_rate_ms`; when a key is pressed it sends
    /// [`Event::Input`], and when the tick interval elapses it sends
    /// [`Event::Tick`]. The returned [`EventHandler`] holds the receiver;
    /// call [`next`](EventHandler::next) in the main loop to receive events.
    ///
    /// # Arguments
    ///
    /// * `tick_rate_ms` - Interval in milliseconds between [`Event::Tick`] emissions.
    ///
    /// # Panics
    ///
    /// The background task may panic if crossterm `poll` or `read` fails (e.g.
    /// terminal disconnected). The main loop does not protect against this.
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        tokio::spawn(async move {
            let tick_rate = Duration::from_millis(tick_rate_ms);
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or(Duration::from_secs(0));
                if event::poll(timeout).expect("Poll failed") {
                    if let CrosstermEvent::Key(key) = event::read().expect("Read failed") {
                        event_tx.send(Event::Input(key)).ok();
                    }
                }
                if last_tick.elapsed() >= tick_rate {
                    event_tx.send(Event::Tick).ok();
                    last_tick = Instant::now();
                }
            }
        });

        Self { tx, rx }
    }

    /// Receives the next event from the channel.
    ///
    /// Returns `None` when all senders have been dropped (e.g. the input task
    /// exited). The main loop typically runs until [`App::should_quit`](crate::app::App::should_quit)
    /// is true, so this is only relevant if the background task is killed.
    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
