use crate::app::{App, ViewMode};
use ratatui::{prelude::*, widgets::*};

pub fn render(f: &mut Frame, app: &App) {
    if app.is_initializing {
        render_loading_screen(f, app);
        return;
    }

    match app.view_mode {
        ViewMode::Dashboard => render_dashboard_view(f, app),
        ViewMode::Spotter => render_spotter_view(f, app),
        ViewMode::Settings => render_settings_view(f, app),
    }
}

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

/// Renders the first-run database initialization screen
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

fn render_settings_view(f: &mut Frame, _app: &App) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded);
    let text = vec![
        Line::from("Config options coming soon..."),
        Line::from("Press '1' for Dashboard, '2' for Spotter"),
    ];

    f.render_widget(
        Paragraph::new(text)
            .block(block)
            .alignment(Alignment::Center),
        f.size(),
    );
}

/// Brand Identity via Colors
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
