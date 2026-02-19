use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the caddy-proxy management submenu popup.
pub fn render_caddy_menu(frame: &mut Frame, area: Rect, app: &App) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Caddy Proxy ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(inner);

    let items = ["Start", "Stop", "Restart"];
    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, &label)| {
            let style = if i == app.caddy_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::REVERSED)
            } else {
                Style::default().fg(Color::White)
            };
            let prefix = if i == app.caddy_selected {
                "> "
            } else {
                "  "
            };
            ListItem::new(format!("{}{}", prefix, label)).style(style)
        })
        .collect();

    let list = List::new(list_items);
    frame.render_widget(list, chunks[0]);

    // Footer hints
    let hints = Line::from(vec![
        Span::styled("\u{2191}\u{2193}", Style::default().fg(Color::Cyan)),
        Span::raw(": navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Cyan)),
        Span::raw(": confirm  "),
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(": cancel"),
    ]);

    let footer = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[1]);
}
