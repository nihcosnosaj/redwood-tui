//! Main entry point for the Redwood TUI application.
//!
//! This module initializes the application, sets up the terminal,
//! creates the event handler, and starts the background API poller.
//! It also handles user input and updates the application state.
use color_eyre::Result;
use crossterm::event::KeyCode;
use ratatui::{backend::CrosstermBackend, Terminal};
use redwood_tui::{
    api::FlightProvider,
    app::{App, ViewMode},
    config, db,
    events::{Event, EventHandler},
    location, logging,
    models::load_aircraft_csv,
    ui,
};
use std::{io, time::Duration, time::Instant};
use tracing::info;

/// Application entry point.
///
/// 1. **Startup**: Load config, initialize logging, install panic hook and
///    color_eyre. Set up the terminal for TUI mode.
/// 2. **Location**: Use IP geolocation or manual config for user coordinates.
/// 3. **App & events**: Create [`App`] and an [`EventHandler`] (tick rate 150 ms).
/// 4. **Background poller**: Spawn a task that periodically fetches flights
///    from OpenSky, enriches them via the local DB, and sends
///    [`Event::FlightUpdate`] on the event channel.
/// 5. **Main loop**: Draw the UI, then block on the next event. Handle input
///    (view switch, quit, delegate to [`App::handle_key`]), ticks
///    ([`App::on_tick`]), and flight updates (sort by distance, update app state).
/// 6. **Shutdown**: Restore terminal and exit.
///
/// # Errors
///
/// Returns an error if terminal setup/restore fails or if color_eyre
/// installation fails.
///
/// # Panics
///
/// Does not panic; a custom panic hook ensures the terminal is restored
/// before the default panic handler runs.
#[tokio::main]
async fn main() -> Result<()> {
    let config = redwood_tui::config::Config::load();
    let _log_guard = logging::initialize_logging();
    install_panic_hook();
    color_eyre::install()?;

    info!("Redwood TUI starting up...");

    let mut terminal = setup_terminal()?;
    // Initialize app: get user coords, create eventhandler, etc.
    let coords = if config.location.auto_gpu {
        redwood_tui::location::get_current_location().await
    } else {
        (config.location.manual_lat, config.location.manual_lon)
    };
    let mut app = App::new();
    app.user_coords = coords;
    app.config = config.clone();
    let events = EventHandler::new(150);

    app.view_mode = match config.ui.default_view.as_str() {
        "Dashboard" => ViewMode::Dashboard,
        "Settings" => ViewMode::Settings,
        _ => ViewMode::Spotter,
    };

    // Background API Poller
    let api_tx = events.tx.clone();
    let poll_interval = config.api.poll_interval_seconds;
    let radius = config.location.detection_radius;
    let user_lat = coords.0;
    let user_lon = coords.1;
    tokio::spawn(async move {
        let provider = FlightProvider::new();
        loop {
            // SF Coordinates
            match provider.fetch_overhead(user_lat, user_lon, radius).await {
                Ok(flights) => {
                    // offload DB lookup to blocking thread
                    let enriched =
                        tokio::task::spawn_blocking(move || db::decorate_flights(flights))
                            .await
                            .unwrap_or_default();

                    let hits = enriched.iter().filter(|f| f.registration.is_some()).count();

                    let _ = api_tx.send(Event::FlightUpdate {
                        flights: enriched,
                        db_hits: hits,
                        timestamp: Instant::now(),
                        is_success: true,
                    });
                }
                Err(e) => {
                    tracing::error!("API Fetch failed: {}", e);
                    let _ = api_tx.send(Event::FlightUpdate {
                        flights: Vec::new(),
                        db_hits: 0,
                        timestamp: std::time::Instant::now(),
                        is_success: false,
                    });
                }
            }
            tokio::time::sleep(Duration::from_secs(poll_interval)).await;
        }
    });

    // Main loop
    let mut event_handler = events;
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(event) = event_handler.next().await {
            match event {
                Event::Input(key) => {
                    match key.code {
                        KeyCode::Char('1') => app.view_mode = ViewMode::Dashboard,
                        KeyCode::Char('2') => app.view_mode = ViewMode::Spotter,
                        KeyCode::Char('3') => app.view_mode = ViewMode::Settings,
                        KeyCode::Char('q') => app.should_quit = true,
                        _ => app.handle_key(key), // Pass other keys to app logic
                    }
                }
                Event::Tick => app.on_tick(),
                Event::FlightUpdate {
                    flights,
                    db_hits,
                    timestamp,
                    is_success,
                } => {
                    if !app.is_initializing {
                        app.last_update_success = is_success;
                        let mut sorted = flights;
                        let (u_lat, u_lon) = app.user_coords;
                        // Sort nearest to farthest
                        sorted.sort_by(|a, b| {
                            a.distance_from(u_lat, u_lon)
                                .partial_cmp(&b.distance_from(u_lat, u_lon))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });

                        if is_success {
                            app.flights = sorted;
                            app.db_match_count = db_hits;
                            app.last_update = Some(timestamp);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

/// Puts the terminal into TUI-friendly mode.
///
/// Enables raw mode (no line buffering, key-by-key input), switches to the
/// alternate screen buffer, and hides the cursor. Must be paired with
/// [`restore_terminal`] on exit so the user's shell is left in a usable state.
///
/// # Errors
///
/// Returns an error if any of the crossterm operations fail.
///
/// # Panics
///
/// Does not panic.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(
        stdout,
        crossterm::terminal::EnterAlternateScreen,
        crossterm::cursor::Hide
    )?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

/// Restores the terminal to normal behavior.
///
/// Disables raw mode, leaves the alternate screen, and shows the cursor.
/// Should be called on normal exit and is also invoked by the panic hook.
///
/// # Errors
///
/// Returns an error if any of the crossterm operations fail.
///
/// # Panics
///
/// Does not panic.
fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    Ok(())
}

/// Installs a custom panic hook that restores the terminal before panicking.
///
/// For a TUI, a panic would otherwise leave the terminal in raw mode and the
/// alternate screen active, which makes the shell hard to use. This hook
/// runs the same cleanup as [`restore_terminal`] (best effort, ignoring
/// errors), then invokes the original panic handler.
///
/// # Panics
///
/// Does not panic. Must be called early in [`main`] so it is in place before
/// any other code can panic.
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Force terminal cleanup!
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show
        )
        .ok();
        original_hook(panic_info);
    }));
}
