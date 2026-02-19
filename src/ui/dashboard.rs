use ratatui::layout::{Alignment, Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::model::{CaddyProxyStatus, ContainerStatus, ServiceSource, View};

/// Render the header bar with caddy-proxy status and view tabs.
pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let caddy_span = match app.caddy_status {
        CaddyProxyStatus::Up => Span::styled(
            " caddy-proxy: \u{25cf} UP ",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
        CaddyProxyStatus::Down => Span::styled(
            " caddy-proxy: \u{25cb} DOWN ",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        CaddyProxyStatus::Unknown => Span::styled(
            " caddy-proxy: ? Unknown ",
            Style::default().fg(Color::Yellow),
        ),
    };

    let project_style = if app.view == View::Project {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let global_style = if app.view == View::Global {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title_line = Line::from(vec![
        Span::styled(" [", Style::default().fg(Color::DarkGray)),
        Span::styled("Project", project_style),
        Span::styled("] [", Style::default().fg(Color::DarkGray)),
        Span::styled("Global", global_style),
        Span::styled("]", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        caddy_span,
    ]);

    let block = Block::default()
        .title(" lcp ")
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let header = Paragraph::new(title_line).block(block);
    frame.render_widget(header, area);
}

/// Render the main service table in the given area.
pub fn render_dashboard(frame: &mut Frame, area: Rect, app: &App) {
    let proxied = app.proxied_services();
    let unproxied = app.unproxied_services();

    let header_cells = ["Domain", "Port", "Status", "TLS", "Source"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header_row = Row::new(header_cells).height(1);

    let mut rows: Vec<Row> = Vec::new();
    let mut row_index: usize = 0;

    // Proxied services
    for svc in &proxied {
        let proxy = svc.proxy.as_ref().unwrap();
        let selected = row_index == app.selected;
        let cursor = if selected { "> " } else { "  " };

        let status_span = status_cell(&svc.status);
        let source_text = source_label(&svc.source);

        let style = if selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        let row = Row::new(vec![
            Cell::from(format!("{}{}", cursor, proxy.domain)),
            Cell::from(proxy.port.to_string()),
            status_span,
            Cell::from(proxy.tls.clone()),
            Cell::from(source_text),
        ])
        .style(style);

        rows.push(row);
        row_index += 1;
    }

    // Separator row
    if !unproxied.is_empty() {
        let sep = Row::new(vec![Cell::from(Line::from(vec![Span::styled(
            "\u{2500}\u{2500} Available (no proxy) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}",
            Style::default().fg(Color::DarkGray),
        )]))])
        .height(1);
        rows.push(sep);
    }

    // Unproxied services
    for svc in &unproxied {
        let selected = row_index == app.selected;
        let cursor = if selected { "> " } else { "  " };

        let port_text = if let Some(&p) = svc.available_ports.first() {
            p.to_string()
        } else {
            "-".to_string()
        };

        let source_text = source_label(&svc.source);

        let style = if selected {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let row = Row::new(vec![
            Cell::from(format!("{}+ {}", cursor, svc.name)),
            Cell::from(port_text),
            Cell::from(""),
            Cell::from(""),
            Cell::from(source_text),
        ])
        .style(style);

        rows.push(row);
        row_index += 1;
    }

    let widths = [
        Constraint::Percentage(33),
        Constraint::Percentage(10),
        Constraint::Percentage(14),
        Constraint::Percentage(14),
        Constraint::Percentage(17),
    ];

    let block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(Color::DarkGray));

    let table = Table::new(rows, &widths)
        .header(header_row)
        .block(block)
        .column_spacing(1);

    frame.render_widget(table, area);
}

/// Render the footer with keybindings.
pub fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let keys = vec![
        Span::styled("[a]", Style::default().fg(Color::Cyan)),
        Span::raw("dd  "),
        Span::styled("[e]", Style::default().fg(Color::Cyan)),
        Span::raw("dit  "),
        Span::styled("[o]", Style::default().fg(Color::Cyan)),
        Span::raw("pen  "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw("efresh  "),
        Span::styled("[c]", Style::default().fg(Color::Cyan)),
        Span::raw("addy  "),
        Span::styled("[?]", Style::default().fg(Color::Cyan)),
        Span::raw("help  "),
        Span::styled("Tab", Style::default().fg(Color::Cyan)),
        Span::raw(": switch view  "),
        Span::styled("[q]", Style::default().fg(Color::Cyan)),
        Span::raw("uit"),
    ];

    let mut line_spans = keys;

    if let Some(ref msg) = app.status_message {
        line_spans.push(Span::raw("  \u{2502} "));
        line_spans.push(Span::styled(
            msg.clone(),
            Style::default().fg(Color::Yellow),
        ));
    }

    let line = Line::from(line_spans);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let footer = Paragraph::new(line).block(block);
    frame.render_widget(footer, area);
}

fn status_cell(status: &ContainerStatus) -> Cell<'static> {
    match status {
        ContainerStatus::Running => Cell::from(Span::styled(
            "\u{25cf} Running",
            Style::default().fg(Color::Green),
        )),
        ContainerStatus::Stopped => Cell::from(Span::styled(
            "\u{25cb} Stopped",
            Style::default().fg(Color::Yellow),
        )),
        ContainerStatus::NotDeployed => Cell::from(Span::styled(
            "- N/A",
            Style::default().fg(Color::DarkGray),
        )),
    }
}

fn source_label(source: &ServiceSource) -> String {
    match source {
        ServiceSource::Compose { file, .. } => {
            file.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "compose".to_string())
        }
        ServiceSource::Runtime => "runtime".to_string(),
    }
}
