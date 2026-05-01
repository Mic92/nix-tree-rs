use anyhow::Result;
use std::io::Write;
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
    let graph = nix_tree::nix::query_path_info(&paths, true, &Default::default()).await?;

    assert!(!graph.paths.is_empty());

    let hello_drv = graph
        .get_path(&paths[0])
        .expect("Should find hello derivation");
    assert!(!hello_drv.references.is_empty());

    let stats = nix_tree::path_stats::calculate_stats(&graph);
    assert!(!stats.is_empty());

    let hello_stats = stats.get(&paths[0]).expect("Should have stats for hello");
    let expected = String::from_utf8(
        Command::new("nix")
            .args([
                "--extra-experimental-features",
                "nix-command",
                "path-info",
                "--closure-size",
                &paths[0],
            ])
            .output()?
            .stdout,
    )?;
    let expected: u64 = expected.split_whitespace().last().unwrap().parse()?;
    assert_eq!(hello_stats.closure_size, expected);

    Ok(())
}

/// https://github.com/Mic92/nix-tree-rs/issues/23
#[tokio::test]
async fn test_unbuilt_derivation_flag() -> Result<()> {
    let mut expr = tempfile::NamedTempFile::with_suffix(".nix")?;
    // Unique salt keeps the output path unbuilt across test runs.
    let salt: u128 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    write!(
        expr,
        r#"derivation {{
  name = "nix-tree-unbuilt-test";
  builder = "/bin/sh";
  args = [ "-c" "echo unreachable > $out" ];
  system = builtins.currentSystem;
  salt = "{salt}";
}}"#
    )?;
    expr.flush()?;

    let opts = nix_tree::nix::QueryOptions {
        file: Some(expr.path().to_string_lossy().into_owned()),
        derivation: true,
        ..Default::default()
    };

    let graph = nix_tree::nix::query_path_info(&[String::new()], true, &opts).await?;

    assert_eq!(graph.roots.len(), 1);
    let root = &graph.roots[0];
    assert!(root.ends_with(".drv"), "expected .drv root, got {root}");

    Ok(())
}
