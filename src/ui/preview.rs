use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::App;
use crate::compose::writer::generate_preview;
use crate::model::ProxyConfig;

/// Render the live YAML preview pane alongside the form.
pub fn render_preview(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Preview ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    // Build a ProxyConfig from form fields for preview
    let service_name = app
        .all_services()
        .get(app.form.service_index)
        .map(|s| s.name.as_str())
        .unwrap_or("service");

    let port: u16 = app.form.port.parse().unwrap_or(0);
    let config = ProxyConfig {
        domain: app.form.domain.clone(),
        port,
        tls: app.form.tls.clone(),
    };

    let preview_text = generate_preview(service_name, &config);

    let paragraph = Paragraph::new(preview_text)
        .block(block)
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
