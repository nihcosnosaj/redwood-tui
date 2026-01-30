use crate::models::Flight;
use crossterm::event::{KeyCode, KeyEvent};

pub struct App {
    pub flights: Vec<Flight>,
    pub selected_index: usize,
    pub tick_count: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            flights: Vec::new(),
            selected_index: 0,
            tick_count: 0,
            should_quit: false,
        }
    }

    pub fn on_tick(&mut self) {
        self.tick_count += 1;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.flights.is_empty() {
                    self.selected_index = (self.selected_index + 1) % self.flights.len();
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.flights.is_empty() {
                    self.selected_index = self.selected_index.checked_sub(1).unwrap_or(self.flights.len() - 1);
                }
            }
            _ => {}
        }
    }
}