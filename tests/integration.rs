use anyhow::Result;
use std::process::Command;

#[tokio::test]
async fn test_parse_hello_derivation() -> Result<()> {
    let output = Command::new("nix-instantiate")
        .arg("<nixpkgs>")
        .arg("-A")
        .arg("hello")
        .output()?;

    if !output.status.success() {
        eprintln!(
            "nix-instantiate failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Failed to instantiate hello derivation");
    }

    let drv_path = String::from_utf8(output.stdout)?.trim().to_string();

    println!("Derivation path: {drv_path}");

    let (hash, name) = nix_tree::store_path::StorePath::parse(&drv_path)?;
    assert_eq!(hash.len(), 32);
    assert!(name.ends_with(".drv"));
    assert!(name.contains("hello"));

    let paths = vec![drv_path];
    let graph = nix_tree::nix::query_path_info(&paths, true, None).await?;

    assert!(!graph.paths.is_empty());

    let hello_drv = graph
        .get_path(&paths[0])
        .expect("Should find hello derivation");
    assert!(!hello_drv.references.is_empty());

    let stats = nix_tree::path_stats::calculate_stats(&graph);
    assert!(!stats.is_empty());

    let hello_stats = stats.get(&paths[0]).expect("Should have stats for hello");
    assert!(hello_stats.closure_size > 0);

    Ok(())
}
