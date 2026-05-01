//! Crude wall-clock benchmark for the list view hot path.
//!
//! Run with: cargo bench --bench scroll -- [installable]
//! Defaults to /run/current-system (or ~/.nix-profile) so it exercises a
//! realistically sized closure.

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use nix_tree::nix::{self, QueryOptions};
use nix_tree::path_stats;
use nix_tree::ui::{self, App};
use ratatui::{Terminal, backend::TestBackend};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let target = std::env::args().skip(1).find(|a| !a.starts_with('-'));
    let paths = match target {
        Some(p) => vec![p],
        None => nix::get_default_roots().await?,
    };

    eprintln!("loading {paths:?} ...");
    let t = Instant::now();
    let graph = nix::query_path_info(&paths, true, &QueryOptions::default()).await?;
    eprintln!(
        "  nix path-info: {:?} ({} paths)",
        t.elapsed(),
        graph.paths.len()
    );

    let t = Instant::now();
    let stats = path_stats::calculate_stats(&graph);
    eprintln!("  calculate_stats: {:?}", t.elapsed());

    let t = Instant::now();
    let mut app = App::new(graph, stats);
    eprintln!("  App::new: {:?}", t.elapsed());

    // Put a large list into the current pane to mimic the laggy case
    // (e.g. referrers of glibc, or search results).
    let mut all: Vec<String> = app.graph.paths.iter().map(|p| p.path.clone()).collect();
    path_stats::sort_paths(&mut all, &app.graph, &app.stats, app.sort_order, None);
    eprintln!("  current pane size: {}", all.len());
    app.current_items = all;
    app.current_state.select(Some(0));

    let mut term = Terminal::new(TestBackend::new(180, 50))?;

    let down = KeyEvent::from(KeyCode::Down);
    let iters = 200u32;

    term.draw(|f| ui::render_frame(f, &app))?;

    let t = Instant::now();
    for _ in 0..iters {
        app.handle_key(down)?;
        term.draw(|f| ui::render_frame(f, &app))?;
    }
    let elapsed = t.elapsed();
    eprintln!(
        "  {iters} x (key + render): {:?} ({:.2} ms/frame)",
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / f64::from(iters)
    );

    Ok(())
}
