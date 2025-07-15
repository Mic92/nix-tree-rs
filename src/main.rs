mod cli;
mod nix;
mod path_stats;
mod store_path;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
};
use std::io;

#[tokio::main]
async fn main() -> Result<()> {
    let config = cli::parse_args()?;

    if config.help {
        cli::print_help();
        return Ok(());
    }

    if config.version {
        cli::print_version();
        return Ok(());
    }

    let paths = if config.paths.is_empty() {
        nix::get_default_roots().await?
    } else {
        config.paths
    };

    println!("Loading store paths...");
    let graph = nix::query_path_info(&paths, true, config.store.as_deref()).await?;

    println!("Calculating sizes...");
    let stats = path_stats::calculate_stats(&graph);

    run_tui(graph, stats).await
}

async fn run_tui(
    graph: store_path::StorePathGraph,
    stats: std::collections::HashMap<String, path_stats::PathStats>,
) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, graph, stats).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    graph: store_path::StorePathGraph,
    stats: std::collections::HashMap<String, path_stats::PathStats>,
) -> Result<()> {
    let mut app = ui::App::new(graph, stats);

    loop {
        terminal.draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(f.area());

            ui::pane::render_panes(f, &app, chunks[0]);
            ui::widgets::render_status_bar(f, &app, chunks[1]);

            if app.show_help {
                ui::widgets::render_help(f, f.area());
            }

            if app.searching {
                ui::widgets::render_search(f, f.area(), &app.search_query);
            }
        })?;

        if let Event::Key(key) = event::read()? {
            if app.handle_key(key)? {
                break;
            }
        }
    }

    Ok(())
}
