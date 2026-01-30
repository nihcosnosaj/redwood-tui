use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use crate::models::Flight;

pub enum Event {
    Tick,
    Input(KeyEvent),
    FlightUpdate(Vec<Flight>),
}

pub struct EventHandler {
    pub tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let event_tx = tx.clone();

        tokio::spawn(async move {
            let tick_rate = Duration::from_millis(tick_rate_ms);
            let mut last_tick = Instant::now();
            loop {
                let timeout = tick_rate.checked_sub(last_tick.elapsed()).unwrap_or(Duration::from_secs(0));
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

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}