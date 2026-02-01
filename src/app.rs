use crate::events::Event;
use crate::models::Flight;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::sync::mpsc;

pub enum InitMessage {
    Progress(f32),
    Done,
    Error(String),
}

// Handles
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ViewMode {
    Dashboard,
    Spotter,
    Settings,
}

impl Default for ViewMode {
    fn default() -> Self {
        ViewMode::Dashboard
    }
}

#[derive(Default)]
pub struct App {
    pub view_mode: ViewMode,
    pub user_coords: (f64, f64),
    pub flights: Vec<Flight>,
    pub selected_index: usize,
    pub tick_count: usize,
    pub should_quit: bool,

    // Init state tracking
    pub is_initializing: bool,
    pub init_progress: f32,
    pub init_message: String,
    pub init_rx: Option<mpsc::Receiver<crate::events::Event>>,

    // System Telemetry info
    pub last_update: Option<std::time::Instant>,
    pub last_update_success: bool,
    pub db_match_count: usize,
}

impl App {
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
        }
    }

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

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Prevent navigation while initializing to avoid data races or confusion
        if self.is_initializing {
            if let KeyCode::Char('q') = key.code {
                self.should_quit = true;
            }
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
}
