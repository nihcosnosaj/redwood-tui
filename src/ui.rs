use crate::app::App;
use ratatui::{prelude::*, widgets::*};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(f.size());

    // --- Sidebar ---
    let items: Vec<ListItem> = app
        .flights
        .iter()
        .enumerate()
        .map(|(i, fl)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!(" ✈  {}", fl.callsign)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Flights Nearby ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, chunks[0]);

    // --- Main Panel ---
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[1]);

    // 1. ASCII Animation
    let frames = ["  ✈   ", "   ✈  ", "    ✈ ", "     ✈", "    ✈ ", "   ✈  "];
    let frame = frames[(app.tick_count / 2) % frames.len()];
    let cloud_art = format!("\n\n   ☁️   {}\n      ☁️      ☁️", frame);

    let anim = Paragraph::new(cloud_art)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .title(" Radar Status ")
                .borders(Borders::ALL),
        );
    f.render_widget(anim, main_chunks[0]);

    // 2. Flight Details
    if let Some(fl) = app.flights.get(app.selected_index) {
        let details = vec![
            Line::from(vec![
                Span::raw("Callsign: "),
                Span::styled(&fl.callsign, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(format!("Origin:   {}", fl.origin_country)),
            Line::from(format!("Altitude: {:.0} m", fl.altitude)),
            Line::from(format!("Speed:    {:.0} km/h", fl.velocity)),
            Line::from(format!("Heading:  {:.0}°", fl.true_track)),
        ];
        let p = Paragraph::new(details).block(
            Block::default()
                .title(" Flight Info ")
                .borders(Borders::ALL)
                .padding(Padding::new(2, 2, 1, 1)),
        );
        f.render_widget(p, main_chunks[1]);
    }
}
