use crossterm::event::{KeyCode, KeyEvent};
use nix_tree::{path_stats, store_path::StorePathGraph, ui::App};

#[tokio::test]
async fn test_initial_dependencies_loading() {
    // First, let's get real data from nix
    let mut paths = vec!["/nix/var/nix/profiles/system".to_string()];

    // Resolve symlinks like the main program does
    for path in &mut paths {
        if !path.starts_with("/nix/store/") {
            if let Ok(resolved) = tokio::fs::canonicalize(&path).await {
                println!("Resolved {} to {}", path, resolved.display());
                *path = resolved.to_string_lossy().to_string();
            }
        }
    }

    let graph = nix_tree::nix::query_path_info(&paths, true, None)
        .await
        .unwrap();

    println!("Graph loaded with {} paths", graph.paths.len());
    println!("Root paths: {:?}", graph.roots);

    // Check if the root path has references
    if let Some(root_path) = graph.get_path(&graph.roots[0]) {
        println!("Root path: {}", root_path.path);
        println!("Root has {} references", root_path.references.len());
        println!("First few references:");
        for (i, reference) in root_path.references.iter().take(5).enumerate() {
            println!("  {}: {}", i, reference);
        }
    }

    // Now test the App initialization
    let stats = path_stats::calculate_stats(&graph);
    let app = App::new(graph.clone(), stats);

    println!("\nApp state after initialization:");
    println!("Active pane: {:?}", app.active_pane);
    println!("Current items: {:?}", app.current_items);
    println!("Current selection: {:?}", app.current_state.selected());
    println!("Next items (dependencies): {} items", app.next_items.len());
    if app.next_items.is_empty() {
        println!("  WARNING: No dependencies loaded!");
    } else {
        println!("  First few dependencies:");
        for (i, dep) in app.next_items.iter().take(5).enumerate() {
            println!("    {}: {}", i, dep);
        }
    }

    // Let's also check what get_references returns directly
    if let Some(selected_path) = app.current_items.first() {
        let refs = graph.get_references(selected_path);
        println!(
            "\nDirect get_references for '{}': {} items",
            selected_path,
            refs.len()
        );
        for (i, ref_path) in refs.iter().take(3).enumerate() {
            println!("  {}: {}", i, ref_path.path);
        }
    }
}

#[test]
fn test_ranger_navigation() {
    // Create a test graph with multiple levels
    let mut graph = StorePathGraph::new();

    // Root -> dep1, dep2
    // dep1 -> dep1-1, dep1-2
    let root = nix_tree::store_path::StorePath {
        path: "/nix/store/aaa-root".to_string(),
        hash: "aaa".to_string(),
        name: "root".to_string(),
        nar_size: 1000,
        closure_size: Some(10000),
        references: vec![
            "/nix/store/bbb-dep1".to_string(),
            "/nix/store/ccc-dep2".to_string(),
        ],
        signatures: vec![],
    };

    let dep1 = nix_tree::store_path::StorePath {
        path: "/nix/store/bbb-dep1".to_string(),
        hash: "bbb".to_string(),
        name: "dep1".to_string(),
        nar_size: 500,
        closure_size: Some(2000),
        references: vec![
            "/nix/store/ddd-dep1-1".to_string(),
            "/nix/store/eee-dep1-2".to_string(),
        ],
        signatures: vec![],
    };

    let dep2 = nix_tree::store_path::StorePath {
        path: "/nix/store/ccc-dep2".to_string(),
        hash: "ccc".to_string(),
        name: "dep2".to_string(),
        nar_size: 300,
        closure_size: Some(300),
        references: vec![],
        signatures: vec![],
    };

    let dep1_1 = nix_tree::store_path::StorePath {
        path: "/nix/store/ddd-dep1-1".to_string(),
        hash: "ddd".to_string(),
        name: "dep1-1".to_string(),
        nar_size: 100,
        closure_size: Some(100),
        references: vec![],
        signatures: vec![],
    };

    let dep1_2 = nix_tree::store_path::StorePath {
        path: "/nix/store/eee-dep1-2".to_string(),
        hash: "eee".to_string(),
        name: "dep1-2".to_string(),
        nar_size: 100,
        closure_size: Some(100),
        references: vec![],
        signatures: vec![],
    };

    graph.add_path(root);
    graph.add_path(dep1);
    graph.add_path(dep2);
    graph.add_path(dep1_1);
    graph.add_path(dep1_2);
    graph.roots = vec!["/nix/store/aaa-root".to_string()];

    let stats = path_stats::calculate_stats(&graph);
    let mut app = App::new(graph, stats);

    // Initial state
    println!("\nInitial state:");
    println!("Active pane: {:?}", app.active_pane);
    println!("Current: {:?}", app.current_items);
    println!("Dependencies: {:?}", app.next_items);
    assert_eq!(
        app.active_pane,
        nix_tree::ui::app::Pane::Current,
        "Should start with Current pane active"
    );
    assert_eq!(app.current_items, vec!["/nix/store/aaa-root"]);
    assert_eq!(app.next_items.len(), 2);

    // Simulate right arrow - should move all dependencies to current
    let right_key = KeyEvent::from(KeyCode::Right);
    app.handle_key(right_key).unwrap();
    println!("\nAfter right arrow:");
    println!("Active pane: {:?}", app.active_pane);
    println!("Current: {:?}", app.current_items);
    println!("Dependencies: {:?}", app.next_items);
    assert_eq!(
        app.active_pane,
        nix_tree::ui::app::Pane::Current,
        "Should stay on Current pane"
    );
    // After right arrow, all dependencies become current items
    assert_eq!(app.current_items.len(), 2);
    assert!(
        app.current_items
            .contains(&"/nix/store/bbb-dep1".to_string())
    );
    assert!(
        app.current_items
            .contains(&"/nix/store/ccc-dep2".to_string())
    );
    // The first item (bbb-dep1) is selected, so its dependencies are shown
    assert_eq!(app.next_items.len(), 2); // dep1-1 and dep1-2
}

#[test]
fn test_update_panes_logic() {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::{Constraint, Layout};

    // Create a test graph with shared dependencies to test added size calculation
    let mut graph = StorePathGraph::new();

    // Root -> dep1, dep2, shared
    // dep1 -> shared, dep1-only
    // dep2 -> shared, dep2-only
    let root = nix_tree::store_path::StorePath {
        path: "/nix/store/abc-root".to_string(),
        hash: "abc".to_string(),
        name: "root".to_string(),
        nar_size: 1000,
        closure_size: Some(5000),
        references: vec![
            "/nix/store/def-dep1".to_string(),
            "/nix/store/ghi-dep2".to_string(),
            "/nix/store/shared".to_string(),
        ],
        signatures: vec![],
    };

    let dep1 = nix_tree::store_path::StorePath {
        path: "/nix/store/def-dep1".to_string(),
        hash: "def".to_string(),
        name: "dep1".to_string(),
        nar_size: 500,
        closure_size: Some(1500),
        references: vec![
            "/nix/store/shared".to_string(),
            "/nix/store/dep1-only".to_string(),
        ],
        signatures: vec![],
    };

    let dep2 = nix_tree::store_path::StorePath {
        path: "/nix/store/ghi-dep2".to_string(),
        hash: "ghi".to_string(),
        name: "dep2".to_string(),
        nar_size: 300,
        closure_size: Some(1300),
        references: vec![
            "/nix/store/shared".to_string(),
            "/nix/store/dep2-only".to_string(),
        ],
        signatures: vec![],
    };

    let shared = nix_tree::store_path::StorePath {
        path: "/nix/store/shared".to_string(),
        hash: "sha".to_string(),
        name: "shared".to_string(),
        nar_size: 200,
        closure_size: Some(200),
        references: vec![],
        signatures: vec![],
    };

    let dep1_only = nix_tree::store_path::StorePath {
        path: "/nix/store/dep1-only".to_string(),
        hash: "d1o".to_string(),
        name: "dep1-only".to_string(),
        nar_size: 100,
        closure_size: Some(100),
        references: vec![],
        signatures: vec![],
    };

    let dep2_only = nix_tree::store_path::StorePath {
        path: "/nix/store/dep2-only".to_string(),
        hash: "d2o".to_string(),
        name: "dep2-only".to_string(),
        nar_size: 150,
        closure_size: Some(150),
        references: vec![],
        signatures: vec![],
    };

    graph.add_path(root);
    graph.add_path(dep1);
    graph.add_path(dep2);
    graph.add_path(shared);
    graph.add_path(dep1_only);
    graph.add_path(dep2_only);
    graph.roots = vec!["/nix/store/abc-root".to_string()];

    // Test with our graph
    let stats = path_stats::calculate_stats(&graph);
    let mut app = App::new(graph.clone(), stats.clone());

    println!("\nTest graph results:");
    println!("Current items: {:?}", app.current_items);
    println!("Next items (dependencies): {:?}", app.next_items);

    assert_eq!(app.current_items.len(), 1);
    assert_eq!(app.current_items[0], "/nix/store/abc-root");
    assert_eq!(app.next_items.len(), 3, "Should have 3 dependencies");

    // Navigate right to move dependencies to current pane
    let right_key = KeyEvent::from(KeyCode::Right);
    app.handle_key(right_key).unwrap();

    // Verify all three dependencies are now current items
    assert_eq!(app.current_items.len(), 3);
    assert!(
        app.current_items
            .contains(&"/nix/store/def-dep1".to_string())
    );
    assert!(
        app.current_items
            .contains(&"/nix/store/ghi-dep2".to_string())
    );
    assert!(app.current_items.contains(&"/nix/store/shared".to_string()));

    // Test rendering the status bar for each item to verify added size calculation
    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    // Select dep1 and check its added size in status bar
    app.current_state.select(Some(
        app.current_items
            .iter()
            .position(|p| p == "/nix/store/def-dep1")
            .unwrap(),
    ));
    app.current_path = Some("/nix/store/def-dep1".to_string());

    terminal
        .draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());
            nix_tree::ui::widgets::render_status_bar(f, &app, chunks[1]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut buffer_text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            buffer_text.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        buffer_text.push('\n');
    }
    println!("\nStatus bar for dep1:");
    println!("{}", buffer_text);

    // Check that the status bar contains the expected added size (600 B)
    // dep1's added size should be: dep1 (500) + dep1-only (100) = 600
    // (shared is not counted as it's also referenced by root and dep2)
    assert!(
        buffer_text.contains("Added Size: 600 B"),
        "dep1 should show added size of 600 B"
    );

    // Select dep2 and check its added size
    app.current_state.select(Some(
        app.current_items
            .iter()
            .position(|p| p == "/nix/store/ghi-dep2")
            .unwrap(),
    ));
    app.current_path = Some("/nix/store/ghi-dep2".to_string());

    terminal
        .draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());
            nix_tree::ui::widgets::render_status_bar(f, &app, chunks[1]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut buffer_text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            buffer_text.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        buffer_text.push('\n');
    }
    println!("\nStatus bar for dep2:");
    println!("{}", buffer_text);

    // Check that the status bar contains the expected added size (450 B)
    // dep2's added size should be: dep2 (300) + dep2-only (150) = 450
    assert!(
        buffer_text.contains("Added Size: 450 B"),
        "dep2 should show added size of 450 B"
    );

    // Select shared and check its added size
    app.current_state.select(Some(
        app.current_items
            .iter()
            .position(|p| p == "/nix/store/shared")
            .unwrap(),
    ));
    app.current_path = Some("/nix/store/shared".to_string());

    terminal
        .draw(|f| {
            let chunks =
                Layout::vertical([Constraint::Min(1), Constraint::Length(4)]).split(f.area());
            nix_tree::ui::widgets::render_status_bar(f, &app, chunks[1]);
        })
        .unwrap();

    let buffer = terminal.backend().buffer();
    let mut buffer_text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            buffer_text.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        buffer_text.push('\n');
    }
    println!("\nStatus bar for shared:");
    println!("{}", buffer_text);

    // Check that the status bar contains the expected added size (0 B)
    // shared's added size should be 0 (referenced by multiple siblings)
    assert!(
        buffer_text.contains("Added Size: 0 B"),
        "shared should show added size of 0 B"
    );
}
