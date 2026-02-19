pub mod caddy_menu;
pub mod dashboard;
pub mod form;
pub mod help;
pub mod preview;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use crate::app::App;
use crate::model::ActiveModal;

/// Top-level draw function — lays out header/table/footer and dispatches modal overlays.
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    dashboard::render_header(frame, chunks[0], app);
    dashboard::render_dashboard(frame, chunks[1], app);
    dashboard::render_footer(frame, chunks[2], app);

    // Render modal overlays on top
    match &app.modal {
        ActiveModal::AddProxy | ActiveModal::EditProxy => {
            let modal_area = centered_rect(90, 60, frame.area());
            let modal_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(modal_area);
            form::render_form(frame, modal_chunks[0], app);
            preview::render_preview(frame, modal_chunks[1], app);
        }
        ActiveModal::CaddyMenu => {
            let area = centered_rect(30, 20, frame.area());
            caddy_menu::render_caddy_menu(frame, area, app);
        }
        ActiveModal::Help => {
            let area = centered_rect(80, 80, frame.area());
            help::render_help(frame, area, app);
        }
        ActiveModal::None => {}
    }
}

/// Returns a centered rect of percent_x wide, percent_y tall within `r`.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
