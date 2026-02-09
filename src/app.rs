//! Application state and UI controller for Redwood TUI.
//!
//! This module defines the main App struct that holds all runtime state
//! (flights, view mode, selection, init progress, etc.) and handles
//! user input and periodic tick updates. It coordinates with the main event loop
//! in `main.rs` and the database intialization worker in `db.rs`.

use crate::config::Config;
use crate::events::Event;
use crate::models::Flight;
use crossterm::event::{KeyCode, KeyEvent};
use std::sync::mpsc;

/// Messages sent during first-run DB initialization.
///
/// Used to communicate progress and completion (or failure) from
/// the DB thread to the main application.
pub enum InitMessage {
    /// Initialization progress from 0.0 to 1.0
    Progress(f32),
    /// DB initialization completed successfully
    Done,
    /// DB initialization failed; payload is error message.
    Error(String),
}

/// Application view mode determining the current screen layout.
///
/// The UI module uses this to choose which view to render.
/// Available views are:
/// - Dashboard: a list of nearby flights with basic info
/// - Spotter: a detailed view of the selected flight
/// - Settings: a screen for configuring app settings (not implemented yet)
#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum ViewMode {
    #[default]
    /// Dashboard: flight list sidebar plus detail and telemetry panel.
    Dashboard,
    /// Radar: view of aircraft tracked on a rudimentary map
    Radar,
    /// Spotter: detailed view of the selected flight.
    Spotter,
    /// Settings: screen for configuring app settings (not implemented yet).
    Settings,
}

/// Main application state and controller.
///
/// Holds all runtime data (flights, user location, selection), view mode,
/// init state and telemetry used by the UI. Updated by the main event loop
/// in response to `Event`s (keyboard, ticks, flight updates)
#[derive(Default)]
pub struct App {
    /// current view mode
    pub view_mode: ViewMode,
    /// user's location as (latitude, longitude) in decimal degrees.
    pub user_coords: (f64, f64),
    /// List of nearby flights sorted by distance from user.
    pub flights: Vec<Flight>,
    /// Index of the selected flight in the flights list.
    pub selected_index: usize,
    /// Number of tick events processed; used for periodic UI updates.
    pub tick_count: usize,
    /// When "true", the main loop exits.
    pub should_quit: bool,

    /// if `true`, the app is still loading the aircraft database ( first-run only).
    pub is_initializing: bool,
    /// progress of database loading from 0.0 to 1.0
    pub init_progress: f32,
    /// message to display during initialization
    pub init_message: String,
    /// channel to receive initialization messages from the DB worker.
    pub init_rx: Option<mpsc::Receiver<crate::events::Event>>,

    /// Timestamp of the last successful flight update.
    pub last_update: Option<std::time::Instant>,
    /// Whether the most recent API/flight update succeeded.
    pub last_update_success: bool,
    /// Number of flights in the current set that were enriched with DB data.
    pub db_match_count: usize,

    /// Loaded configuration; used by Settings view and saved to config.toml on Save.
    pub config: Config,
    /// Index of the selected setting row in the Settings view (0..=5).
    pub settings_selected_index: usize,
    /// Brief message shown in Settings after save (e.g. "Config saved.").
    pub settings_message: Option<String>,
    /// region we are tracking.
    pub tracking_region: String,
}

impl App {
    /// Creates a new application instance.
    ///
    /// If `opensky_aircraft.db` does not exist, starts database initialization
    /// in a background thread and sets `is_initializing` to `true` and
    /// `init_rx` to the receiver for progress/done/error events. Otherwise
    /// the app starts in a ready state with no init receiver.
    ///
    /// # Panics
    ///
    /// Does not panic. Database init failures are reported via `Event::DbError`.
    pub fn new() -> Self {
        let db_exists = std::path::Path::new("opensky_aircraft.db").exists();
        let (is_initializing, init_rx) = if !db_exists {
            let (tx, rx) = mpsc::channel();
            crate::db::init_database(tx); // We'll define this below
            (true, Some(rx))
        } else {
            (false, None)
        };

        Self {
            view_mode: ViewMode::Dashboard,
            user_coords: (0.0, 0.0),
            flights: Vec::new(),
            selected_index: 0,
            tick_count: 0,
            should_quit: false,
            is_initializing,
            init_progress: 0.0,
            init_message: "Initializing database...".to_string(),
            init_rx,
            last_update: None,
            db_match_count: 0,
            last_update_success: false,
            config: Config::default(),
            settings_selected_index: 0,
            settings_message: None,
            tracking_region: "Unknown".to_string(),
        }
    }

    /// Processes a single tick from the event loop.
    ///
    /// Increments `tick_count` and drains any pending init events from the DB thread.
    /// Updates `init_progress` and `init_message` accordingly.
    /// Sets `is_initializing` to `false` and `init_rx` to `None` when initialization completes.
    /// Sets `init_message` to the error message if initialization fails.
    pub fn on_tick(&mut self) {
        self.tick_count += 1;

        let mut should_cleanup = false;

        // Catch messages from the DB worker
        if let Some(ref rx) = self.init_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    Event::DbProgress(p) => self.init_progress = p,
                    Event::DbDone => {
                        self.is_initializing = false;
                        should_cleanup = true;
                    }
                    Event::DbError(e) => {
                        self.init_message = e;
                        should_cleanup = true;
                    }
                    _ => {}
                }
            }
            if should_cleanup {
                self.init_rx = None;
            }
        }
    }

    /// Handles a keyboard event from the user.
    ///
    /// If the app is initializing, only allows quitting with 'q'.
    /// Otherwise, handles navigation (up/down), selection, and quitting.
    /// Exits the app with 'q' or when the flights list is empty.
    /// Navigates up/down in the flight list using 'k'/'j' or arrow keys.
    /// Selects the next/previous flight in the list.
    /// Quits the app with 'q'.
    ///
    /// # Panics
    ///
    /// Does not panic. Index out of bounds is handled by wrapping around.
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Prevent navigation while initializing to avoid data races or confusion
        if self.is_initializing {
            if let KeyCode::Char('q') = key.code {
                self.should_quit = true;
            }
            return;
        }

        if self.view_mode == ViewMode::Settings {
            self.handle_settings_key(key);
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.flights.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.flights.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.flights.is_empty() {
                    self.selected_index = self
                        .selected_index
                        .checked_sub(1)
                        .unwrap_or(self.flights.len() - 1);
                }
            }
            _ => {}
        }
    }

    const SETTINGS_FIELD_COUNT: usize = 6;

    /// Handles key input when the Settings view is active.
    fn handle_settings_key(&mut self, key: KeyEvent) {
        use KeyCode::*;
        self.settings_message = None;

        match key.code {
            Up | Char('k') => {
                self.settings_selected_index = self
                    .settings_selected_index
                    .checked_sub(1)
                    .unwrap_or(Self::SETTINGS_FIELD_COUNT - 1);
            }
            Down | Char('j') => {
                self.settings_selected_index =
                    (self.settings_selected_index + 1) % Self::SETTINGS_FIELD_COUNT;
            }
            Char('s') => {
                if let Err(e) = self.config.save() {
                    self.settings_message = Some(format!("Save failed: {}", e));
                } else {
                    self.settings_message =
                        Some("Config saved. Restart for poll/radius changes.".to_string());
                    self.user_coords = if self.config.location.auto_gpu {
                        self.user_coords
                    } else {
                        (
                            self.config.location.manual_lat,
                            self.config.location.manual_lon,
                        )
                    };
                    self.view_mode = match self.config.ui.default_view.as_str() {
                        "Dashboard" => ViewMode::Dashboard,
                        "Settings" => ViewMode::Settings,
                        _ => ViewMode::Spotter,
                    };
                }
            }
            Enter | Char(' ') => match self.settings_selected_index {
                0 => self.config.location.auto_gpu = !self.config.location.auto_gpu,
                5 => {
                    self.config.ui.default_view = match self.config.ui.default_view.as_str() {
                        "Dashboard" => "Spotter".to_string(),
                        "Spotter" => "Settings".to_string(),
                        _ => "Dashboard".to_string(),
                    };
                }
                _ => {}
            },
            Char('+') | Char('=') => self.settings_increment(),
            Char('-') => self.settings_decrement(),
            _ => {}
        }
    }

    fn settings_increment(&mut self) {
        match self.settings_selected_index {
            1 => {
                self.config.location.manual_lat = (self.config.location.manual_lat + 0.1).min(90.0)
            }
            2 => {
                self.config.location.manual_lon = (self.config.location.manual_lon + 0.1).min(180.0)
            }
            3 => {
                self.config.location.detection_radius =
                    (self.config.location.detection_radius + 5.0).min(500.0)
            }
            4 => {
                self.config.api.poll_interval_seconds =
                    (self.config.api.poll_interval_seconds + 5).min(600)
            }
            _ => {}
        }
    }

    fn settings_decrement(&mut self) {
        match self.settings_selected_index {
            1 => {
                self.config.location.manual_lat = (self.config.location.manual_lat - 0.1).max(-90.0)
            }
            2 => {
                self.config.location.manual_lon =
                    (self.config.location.manual_lon - 0.1).max(-180.0)
            }
            3 => {
                self.config.location.detection_radius =
                    (self.config.location.detection_radius - 5.0).max(1.0)
            }
            4 => {
                self.config.api.poll_interval_seconds = self
                    .config
                    .api
                    .poll_interval_seconds
                    .saturating_sub(5)
                    .max(5)
            }
            _ => {}
        }
    }
}
