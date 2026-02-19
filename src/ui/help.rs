use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::App;

/// Render the help overlay with all keybindings.
pub fn render_help(frame: &mut Frame, area: Rect, _app: &App) {
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Help \u{2014} lcp ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let key_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::White);
    let sep_style = Style::default().fg(Color::DarkGray);

    let lines = vec![
        Line::from(vec![
            Span::styled("  Key          ", key_style),
            Span::styled("Action", desc_style),
        ]),
        Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            sep_style,
        )),
        help_line("  Tab          ", "Switch Project/Global view", key_style, desc_style),
        help_line("  j / \u{2193}        ", "Move down", key_style, desc_style),
        help_line("  k / \u{2191}        ", "Move up", key_style, desc_style),
        help_line("  g            ", "Jump to top", key_style, desc_style),
        help_line("  G            ", "Jump to bottom", key_style, desc_style),
        help_line("  a            ", "Add proxy to service", key_style, desc_style),
        help_line("  e            ", "Edit proxy config", key_style, desc_style),
        help_line("  o            ", "Open in browser (https)", key_style, desc_style),
        help_line("  r            ", "Refresh services", key_style, desc_style),
        help_line("  c            ", "Caddy-proxy management", key_style, desc_style),
        help_line("  ?            ", "Help", key_style, desc_style),
        help_line("  q / Esc      ", "Quit / Close modal", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled(
            "  \u{2500}\u{2500}\u{2500} In form \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            sep_style,
        )),
        help_line("  Tab          ", "Next field", key_style, desc_style),
        help_line("  Shift+Tab    ", "Previous field", key_style, desc_style),
        help_line("  Enter        ", "Confirm / Save", key_style, desc_style),
        help_line("  Esc          ", "Cancel", key_style, desc_style),
        Line::from(""),
        Line::from(Span::styled(
            "  Press Esc or ? to close this help.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn help_line<'a>(key: &'a str, desc: &'a str, key_style: Style, desc_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(key, key_style),
        Span::styled(desc, desc_style),
    ])
}
