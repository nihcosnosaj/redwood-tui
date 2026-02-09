//! TUI rendering for the Redwood TUI
//!
//! This module handles all UI rendering logic using the `ratatui` crate,
//! including dashboard views, spotter views, loading screens, and settings.

use crate::app::{App, ViewMode};
use ratatui::{
    prelude::*,
    widgets::{canvas::*, *}, // Imports Points, Circle, Map, etc.
};

use ratatui::text::Line;

/// Renders one frame of the TUI based on current application state.
///
/// If the app is still initializing the aircraft database, draws the loading
/// screen (progress gauge and message). Otherwise selects the view from
/// [`App::view_mode`]: dashboard (list + detail + telemetry), spotter
/// (focused aircraft ID), or settings placeholder.
///
/// # Arguments
///
/// * `f` - The ratatui frame to draw into (from `terminal.draw()`).
/// * `app` - Current application state (flights, selection, view mode, etc.).
pub fn render(f: &mut Frame, app: &App) {
    if app.is_initializing {
        render_loading_screen(f, app);
        return;
    }

    match app.view_mode {
        ViewMode::Dashboard => render_dashboard_view(f, app),
        ViewMode::Spotter => render_spotter_view(f, app),
        ViewMode::Settings => render_settings_view(f, app),
        ViewMode::Radar => render_radar_view(f, app),
    }
}

/// Dashboard view: flight list sidebar (30%) + main area (70%).
///
/// The main area is split into a fixed-height telemetry block and a details
/// paragraph. Shows "Flights Nearby" list, system telemetry (network, latency,
/// DB hits, selected ICAO, enriched vs raw), and detailed identity/telemetry
/// for the selected flight.
///
/// # Arguments
///
/// * `f` - The ratatui frame to draw into (from `terminal.draw()`).
/// * `app` - Current application state (flights, selection, view mode, etc.).
fn render_dashboard_view(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.size());

    // Sidebar
    let items: Vec<ListItem> = app
        .flights
        .iter()
        .enumerate()
        .map(|(i, fl)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Rgb(30, 30, 60))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            // Use Callsign if it's not "N/A", otherwise fall back to Registration
            let id = if fl.callsign != "N/A" && !fl.callsign.is_empty() {
                &fl.callsign
            } else {
                fl.registration.as_deref().unwrap_or("Unknown")
            };

            // Get a short version of the operator for the list
            let op = fl.operator.as_deref().unwrap_or("???");
            let short_op = if op.len() > 12 { &op[..12] } else { op };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<8}", id), style),
                Span::styled(
                    format!(" │ {}", short_op),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Flights Nearby ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, chunks[0]);

    // Main Panel
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[1]);

    // System Telemetry Panel
    if let Some(fl) = app.flights.get(app.selected_index) {
        let now = std::time::Instant::now();
        let seconds_ago = app
            .last_update
            .map(|inst| now.duration_since(inst).as_secs())
            .unwrap_or(0);

        // Color code the "Freshness"
        let latency_color = if seconds_ago < 40 {
            Color::Green
        } else {
            Color::Red
        };

        let stats_content = vec![
            Line::from(vec![
                Span::styled("  NETWORK: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled("ONLINE", Style::default().fg(Color::Green)),
                Span::raw("  │  "),
                Span::styled("LATENCY: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{}s", seconds_ago),
                    Style::default().fg(latency_color),
                ),
                Span::raw("  │  "),
                Span::styled("DB HITS: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("{}/{}", app.db_match_count, app.flights.len()),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(""), // Spacer
            Line::from(vec![
                Span::styled(
                    "  SELECTED: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(&fl.icao24, Style::default().fg(Color::Yellow)),
                Span::raw("  │  "),
                Span::styled("TRACKING: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(if fl.registration.is_some() {
                    "ENRICHED"
                } else {
                    "RAW DATA"
                }),
            ]),
            Line::from(vec![
                Span::styled("  BASE: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&app.tracking_region, Style::default().fg(Color::Magenta)),
                Span::raw("  │  "),
                Span::styled("RANGE: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{}km", app.config.location.detection_radius)),
            ]),
        ];

        let stats_block = Paragraph::new(stats_content)
            .block(
                Block::default()
                    .title(" System Telemetry ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .alignment(Alignment::Left);

        f.render_widget(stats_block, main_chunks[0]);
    }

    // Flight Details
    if let Some(fl) = app.flights.get(app.selected_index) {
        let operator = fl.operator.as_deref().unwrap_or("Private/Unknown");
        let op_color = get_operator_color(operator);

        let details = vec![
            Line::from(vec![
                Span::styled(
                    "Registration: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    fl.registration.as_deref().unwrap_or("N/A"),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("  |  "),
                Span::styled("Callsign: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&fl.callsign, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Airline:      ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(operator, Style::default().fg(op_color)),
            ]),
            Line::from(vec![
                Span::styled(
                    "Manufacturer: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(fl.manufacturer.as_deref().unwrap_or("Unknown")),
            ]),
            Line::from(vec![
                Span::styled(
                    "Model:        ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(fl.model.as_deref().unwrap_or("Unknown Aircraft")),
                Span::raw(format!(
                    " ({})",
                    fl.aircraft_type.as_deref().unwrap_or("---")
                )),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "Telemetry:    ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(
                    "{:.0} m  |  {:.0} km/h  |  {:.0}°",
                    fl.altitude, fl.velocity, fl.true_track
                )),
            ]),
            Line::from(vec![
                Span::styled(
                    "Origin:       ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(&fl.origin_country),
            ]),
        ];

        let p = Paragraph::new(details).block(
            Block::default()
                .title(" Detailed Aircraft Identity ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1)),
        );
        f.render_widget(p, main_chunks[1]);
    }
}

fn render_radar_view(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    draw_flight_sidebar(f, app, chunks[0]);

    let (u_lat, u_lon) = app.user_coords;
    let radius = 1.0; // Your zoom level

    let radar_canvas = Canvas::default()
        .block(Block::bordered().title(" Precision Radar "))
        .marker(symbols::Marker::Braille)
        .x_bounds([u_lon - radius, u_lon + radius])
        .y_bounds([u_lat - radius, u_lat + radius])
        .paint(|ctx| {
            // Landmass Outlines 
            ctx.draw(&Map {
                color: Color::Rgb(50, 50, 50),   // Dark grey for a "tactical" look
                resolution: MapResolution::High, // Uses high-res coastline data
            });

            // Orientation Markers (N, S, E, W)
            let label_style = Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM);

            // North
            ctx.print(
                u_lon,
                u_lat + (radius * 0.9),
                Line::from(Span::styled("N", label_style)),
            );
            // South
            ctx.print(
                u_lon,
                u_lat - (radius * 0.9),
                Line::from(Span::styled("S", label_style)),
            );
            // East
            ctx.print(
                u_lon + (radius * 0.9),
                u_lat,
                Line::from(Span::styled("E", label_style)),
            );
            // West
            ctx.print(
                u_lon - (radius * 0.9),
                u_lat,
                Line::from(Span::styled("W", label_style)),
            );

            // Aircraft Rendering 
            for (i, flight) in app.flights.iter().enumerate() {
                let is_selected = i == app.selected_index;

                if is_selected {
                    ctx.print(
                        flight.longitude,
                        flight.latitude,
                        Line::from(vec![
                            Span::styled(
                                " ✈ ",
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            ),
                            Span::styled(
                                format!(" {} ", flight.callsign),
                                Style::default().fg(Color::Black).bg(Color::Yellow),
                            ),
                        ]),
                    );
                } else {
                    ctx.print(flight.longitude, flight.latitude, "·");
                }
            }

            // --- 4. Home Crosshair ---
            ctx.print(
                u_lon,
                u_lat,
                Line::from(Span::styled(" ⌖ ", Style::default().fg(Color::Cyan))),
            );
        });

    f.render_widget(radar_canvas, chunks[1]);
}

/// Spotter view: centered aircraft identity with operator, callsign, model.
///
/// Uses 20% / 60% / 20% vertical chunks. The middle chunk shows the selected
/// flight's operator (styled), callsign (inverse), and model. The bottom
/// chunk shows altitude, velocity, and heading.
///
/// # Arguments
///
/// * `f` - The ratatui frame to draw into (from `terminal.draw()`).
/// * `app` - Current application state (flights, selection, view mode, etc.).
fn render_spotter_view(f: &mut Frame, app: &App) {
    let area = f.size();

    if let Some(target) = app.flights.get(app.selected_index) {
        let chunks = Layout::default()
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(area);
        // ID Block - the big center block.
        let id_text = vec![
            Line::from(Span::styled(
                target.operator.as_deref().unwrap_or("Unknown Operator"),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!(" {} ", target.callsign),
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(target.model.as_deref().unwrap_or("Unknown Aircraft")),
        ];

        f.render_widget(
            Paragraph::new(id_text)
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::NONE)),
            chunks[1],
        );

        // Telemetry - the bottom bar
        let telemetry = Paragraph::new(format!(
            "Altitude: {} m | Velocity: {} km/h | Heading: {}°",
            target.altitude, target.velocity, target.true_track
        ))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));

        f.render_widget(telemetry, chunks[2]);
    }
}

/// Renders the first-run database initialization screen.
///
/// Shows a centered progress gauge (from `app.init_progress`, 0.0–1.0) and
/// the status message (`app.init_message`). Used when `app.is_initializing`
/// is true.
fn render_loading_screen(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height / 2 - 3),
            Constraint::Length(3), // Progress bar
            Constraint::Length(1), // Message
            Constraint::Min(0),
        ])
        .split(area);

    let gauge = Gauge::default()
        .block(
            Block::default()
                .title(" Initializing Aircraft Database ")
                .borders(Borders::ALL),
        )
        .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 20, 20)))
        .percent((app.init_progress * 100.0) as u16);

    f.render_widget(gauge, chunks[1]);
    let msg = Paragraph::new(app.init_message.as_str())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(msg, chunks[2]);
}

/// Settings view: displays config and allows editing with ↑/↓, Enter/Space, +/-.
fn render_settings_view(f: &mut Frame, app: &App) {
    let area = f.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(" Settings ")
        .style(Style::default().add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    let c = &app.config;
    let sel = app.settings_selected_index;
    let rows: [(usize, &str, String); 6] = [
        (
            0,
            "Use IP geolocation     ",
            if c.location.auto_gpu { "Yes" } else { "No" }.to_string(),
        ),
        (
            1,
            "Manual latitude        ",
            format!("{:.4}", c.location.manual_lat),
        ),
        (
            2,
            "Manual longitude       ",
            format!("{:.4}", c.location.manual_lon),
        ),
        (
            3,
            "Detection radius (km)  ",
            format!("{:.0}", c.location.detection_radius),
        ),
        (
            4,
            "Poll interval (s)      ",
            c.api.poll_interval_seconds.to_string(),
        ),
        (5, "Default view          ", c.ui.default_view.clone()),
    ];
    let items: Vec<Line> = rows
        .iter()
        .map(|(idx, label, value)| {
            let style = if *idx == sel {
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Rgb(30, 30, 60))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Line::from(vec![
                Span::styled(format!("  {} ", label), style),
                Span::styled(value.as_str(), style),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" config.toml ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let inner = block.inner(chunks[1]);
    f.render_widget(block, chunks[1]);

    let list = Paragraph::new(items).alignment(Alignment::Left);
    f.render_widget(list, inner);

    let help = Paragraph::new(vec![
        Line::from(" ↑/↓ select   Enter/Space toggle or cycle   +/- change number   s Save   q back  1/2/3 views"),
    ])
    .style(Style::default().fg(Color::DarkGray))
    .alignment(Alignment::Center);
    f.render_widget(help, chunks[2]);

    if let Some(ref msg) = app.settings_message {
        let p = Paragraph::new(msg.as_str())
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        f.render_widget(p, chunks[3]);
    }
}

fn draw_flight_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .flights
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = if Some(i) == Some(app.selected_index) {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(format!(" > {}", f.callsign)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title("Flights"))
        .highlight_symbol(">> ");

    f.render_widget(list, area);
}

/// Returns a color associated with the operator name for brand-style display.
///
/// Matches common US airlines and cargo operators by substring (case-insensitive).
/// Unknown operators return [`Color::White`].
fn get_operator_color(operator: &str) -> Color {
    let op = operator.to_lowercase();
    if op.contains("united") {
        Color::Blue
    } else if op.contains("southwest") {
        Color::Yellow
    } else if op.contains("delta") {
        Color::Rgb(180, 20, 40)
    } else if op.contains("american") {
        Color::Cyan
    } else if op.contains("alaska") {
        Color::Rgb(0, 66, 110)
    } else if op.contains("fedex") {
        Color::Magenta
    } else if op.contains("ups") {
        Color::Rgb(80, 40, 0)
    } else {
        Color::White
    }
}
