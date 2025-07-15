mod cli;
mod nix;
mod path_stats;
mod store_path;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
};
use std::io;
use std::time::Duration;

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

    let mut paths = if config.paths.is_empty() {
        nix::get_default_roots().await?
    } else {
        config.paths
    };

    // Resolve symlinks for paths outside the Nix store
    for path in &mut paths {
        if !path.starts_with("/nix/store/") {
            if let Ok(resolved) = tokio::fs::canonicalize(&path).await {
                *path = resolved.to_string_lossy().to_string();
            }
        }
    }

    println!("Loading store paths...");
    let graph = nix::query_path_info(
        &paths,
        true,
        config.store.as_deref(),
        &config.nix_options,
        config.file.as_deref(),
    )
    .await?;

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
    let mut needs_render = true;

    loop {
        // Only render when needed
        if needs_render {
            terminal.draw(|f| {
                let chunks =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());

                ui::pane::render_panes(f, &app, chunks[0]);
                ui::widgets::render_status_bar(f, &app, chunks[1]);

                if app.show_help {
                    ui::widgets::render_help(f, f.area());
                }

                if app.searching {
                    ui::widgets::render_search(f, f.area(), &app.search_query);
                }

                // Render modal on top
                ui::widgets::render_modal(f, &app, f.area());
            })?;
            needs_render = false;
        }

        // Poll with a shorter timeout for better responsiveness
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                // Only handle key press events, ignore key release events
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_key(key)? {
                        break;
                    }
                    needs_render = true;

                    // Process at most 10 additional events per frame to reduce jumpiness
                    let mut events_processed = 0;
                    while events_processed < 10 && event::poll(Duration::from_millis(0))? {
                        match event::read()? {
                            Event::Key(k) if k.kind == KeyEventKind::Press => {
                                // Process the additional key event
                                if app.handle_key(k)? {
                                    return Ok(());
                                }
                                events_processed += 1;
                            }
                            Event::Resize(_, _) => needs_render = true,
                            _ => {}
                        }
                    }
                }
                Event::Resize(_, _) => {
                    needs_render = true;
                }
                _ => {}
            }
        }
    }

    Ok(())
}
