pub mod app;
pub mod pane;
pub mod widgets;

pub use app::App;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};

/// Single source of truth for the per-frame layout so the live TUI, the
/// snapshot example and the scroll bench cannot drift apart.
pub fn render_frame(f: &mut Frame, app: &App) {
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());

    pane::render_panes(f, app, chunks[0]);
    widgets::render_status_bar(f, app, chunks[1]);

    if app.show_help {
        widgets::render_help(f, f.area());
    }
    if app.searching {
        widgets::render_search(f, f.area(), &app.search_query);
    }
    widgets::render_modal(f, app, f.area());
}
