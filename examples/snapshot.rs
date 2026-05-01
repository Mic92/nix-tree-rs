//! Render the TUI to a TestBackend and dump it as plain text so the layout
//! can be inspected without an interactive terminal.
//!
//!   cargo run --example snapshot -- [installable] [keys]
//!
//! `keys` is a string of single-char key presses applied before the snapshot,
//! e.g. `jjjl` or `?` or `/glibc<enter>` (use `\n` for Enter).

use crossterm::event::{KeyCode, KeyEvent};
use nix_tree::nix::{self, QueryOptions};
use nix_tree::path_stats;
use nix_tree::ui::{self, App, pane};
use ratatui::layout::{Constraint, Layout};
use ratatui::{Terminal, backend::TestBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let target = args.next();
    let keys = args.next().unwrap_or_default();

    let paths = match target {
        Some(p) => vec![p],
        None => nix::get_default_roots().await?,
    };

    let graph = nix::query_path_info(&paths, true, &QueryOptions::default()).await?;
    let stats = path_stats::calculate_stats(&graph);
    let mut app = App::new(graph, stats);

    for c in keys.chars() {
        let code = match c {
            '\n' => KeyCode::Enter,
            c => KeyCode::Char(c),
        };
        app.handle_key(KeyEvent::from(code))?;
    }

    let mut term = Terminal::new(TestBackend::new(140, 40))?;
    term.draw(|f| {
        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());
        pane::render_panes(f, &app, chunks[0]);
        ui::widgets::render_status_bar(f, &app, chunks[1]);
        if app.show_help {
            ui::widgets::render_help(f, f.area());
        }
        if app.searching {
            ui::widgets::render_search(f, f.area(), &app.search_query);
        }
        ui::widgets::render_modal(f, &app, f.area());
    })?;

    let buf = term.backend().buffer();
    for y in 0..buf.area.height {
        let mut line = String::new();
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        println!("{}", line.trim_end());
    }
    Ok(())
}
