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
    let events = EventHandler::new(150);

    app.view_mode = match config.ui.default_view.as_str() {
        "Dashboard" => ViewMode::Dashboard,
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
            if let Ok(flights) = provider.fetch_overhead(user_lat, user_lon, radius).await {
                // offload DB lookup to blocking thread
                let enriched = tokio::task::spawn_blocking(move || db::decorate_flights(flights))
                    .await
                    .unwrap_or_default();

                let hits = enriched.iter().filter(|f| f.registration.is_some()).count();

                let _ = api_tx.send(Event::FlightUpdate {
                    flights: enriched,
                    db_hits: hits,
                    timestamp: Instant::now(),
                });
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
                        KeyCode::Char('q') => app.should_quit = true,
                        _ => app.handle_key(key), // Pass other keys to app logic
                    }
                }
                Event::Tick => app.on_tick(),
                Event::FlightUpdate {
                    flights,
                    db_hits,
                    timestamp,
                } => {
                    if !app.is_initializing {
                        let mut sorted = flights;
                        let (u_lat, u_lon) = app.user_coords;
                        // Sort nearest to farthest
                        sorted.sort_by(|a, b| {
                            a.distance_from(u_lat, u_lon)
                                .partial_cmp(&b.distance_from(u_lat, u_lon))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });

                        app.flights = sorted;
                        app.db_match_count = db_hits;
                        app.last_update = Some(timestamp);
                    }
                }
                _ => {}
            }
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

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

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;
    Ok(())
}

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
