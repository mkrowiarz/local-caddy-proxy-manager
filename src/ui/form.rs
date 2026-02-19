use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::model::ActiveModal;

/// Render the add/edit proxy popup form.
pub fn render_form(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let title = match app.modal {
        ActiveModal::AddProxy => " Add Proxy ",
        ActiveModal::EditProxy => " Edit Proxy ",
        _ => " Proxy ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Split inner area into field rows + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Domain
            Constraint::Length(3), // Port
            Constraint::Length(3), // TLS
            Constraint::Min(0),   // spacer
            Constraint::Length(2), // footer hints
        ])
        .split(inner);

    let fields = [
        ("Domain", &app.form.domain),
        ("Port", &app.form.port),
        ("TLS", &app.form.tls),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let focused = app.form.focused_field == i;

        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let label_style = if focused {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let display_value = if focused {
            format!("{}_", value)
        } else {
            value.to_string()
        };

        let field_block = Block::default()
            .title(Span::styled(format!(" {} ", label), label_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        let input = Paragraph::new(display_value).block(field_block);
        frame.render_widget(input, chunks[i]);
    }

    // Footer hints
    let hints = Line::from(vec![
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": next  "),
        Span::styled("S-Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": prev  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": save  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": cancel"),
    ]);

    let footer = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[4]);
}
