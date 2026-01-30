use color_eyre::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};
use redwood_tui::{api::FlightProvider, app::App, events::{Event, EventHandler}, ui, logging};
use tracing::{info, error, debug};


#[tokio::main]
async fn main() -> Result<()> {
    // Instrumentation and safety
    let _log_guard = logging::initialize_logging();
    install_panic_hook();
    color_eyre::install()?;


    // Ready terminal and state
    let mut terminal = setup_terminal()?;
    let mut app = App::new();
    let events = EventHandler::new(150); // High tick rate for smooth animation
    
    // Background API Poller
    let api_tx = events.tx.clone();
    tokio::spawn(async move {
        let provider = FlightProvider::new();
        loop {
            // Hardcoded SF coordinates for my specific needs -- i live in sf!
            if let Ok(flights) = provider.fetch_overhead(37.77, -122.41).await {
                let _ = api_tx.send(Event::FlightUpdate(flights));
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    // Main loop
    let mut event_handler = events;
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(event) = event_handler.next().await {
            match event {
                Event::Tick => app.on_tick(),
                Event::Input(key) => app.handle_key(key),
                Event::FlightUpdate(f) => app.flights = f,
            }
        }
    }

    restore_terminal(terminal)?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen, crossterm::cursor::Hide)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen, crossterm::cursor::Show)?;
    Ok(())
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Force terminal cleanup!
        crossterm::terminal::disable_raw_mode().ok();
        crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen, crossterm::cursor::Show).ok();
        original_hook(panic_info);
    }));
}