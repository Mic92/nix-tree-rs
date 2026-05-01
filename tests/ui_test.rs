use crossterm::event::{KeyCode, KeyEvent};
use nix_tree::{
    path_stats,
    store_path::{StorePath, StorePathGraph},
    ui::App,
};

fn sp(name: &str, nar_size: u64, refs: &[&str]) -> StorePath {
    let path = format!("/nix/store/{:a<32}-{name}", "");
    StorePath {
        hash: "a".repeat(32),
        name: name.to_string(),
        nar_size,
        references: refs
            .iter()
            .map(|r| format!("/nix/store/{:a<32}-{r}", ""))
            .collect(),
        signatures: vec![],
        path,
    }
}

fn graph(paths: Vec<StorePath>, root: &str) -> StorePathGraph {
    let mut g = StorePathGraph::new();
    let root_path = paths
        .iter()
        .find(|p| p.name == root)
        .map(|p| p.path.clone())
        .unwrap();
    for p in paths {
        g.add_path(p);
    }
    g.roots = vec![root_path];
    g
}

#[test]
fn ranger_navigation() {
    let g = graph(
        vec![
            sp("root", 1000, &["dep1", "dep2"]),
            sp("dep1", 500, &["leaf1", "leaf2"]),
            sp("dep2", 300, &[]),
            sp("leaf1", 100, &[]),
            sp("leaf2", 100, &[]),
        ],
        "root",
    );
    let stats = path_stats::calculate_stats(&g);
    let mut app = App::new(g, stats);

    assert_eq!(app.current_items.len(), 1);
    assert_eq!(app.next_items.len(), 2);

    app.handle_key(KeyEvent::from(KeyCode::Right)).unwrap();

    assert_eq!(app.current_items.len(), 2);
    // dep1 has the larger closure → sorted first → its leaves populate next.
    assert_eq!(app.next_items.len(), 2);
}

/// Added size = what disappears from the parent closure if this node alone is
/// removed. Shared deps must not be double-counted.
#[test]
fn added_size_excludes_shared_deps() {
    let g = graph(
        vec![
            sp("root", 1000, &["dep1", "dep2", "shared"]),
            sp("dep1", 500, &["shared", "only1"]),
            sp("dep2", 300, &["shared", "only2"]),
            sp("shared", 200, &[]),
            sp("only1", 100, &[]),
            sp("only2", 150, &[]),
        ],
        "root",
    );
    let stats = path_stats::calculate_stats(&g);
    let mut app = App::new(g, stats);

    app.handle_key(KeyEvent::from(KeyCode::Right)).unwrap();

    let p = |n: &str| format!("/nix/store/{:a<32}-{n}", "");
    assert_eq!(app.added_size_of(&p("dep1")), 600); // dep1 + only1; shared survives via dep2
    assert_eq!(app.added_size_of(&p("dep2")), 450); // dep2 + only2
    assert_eq!(app.added_size_of(&p("shared")), 200); // leaf: just itself
}
