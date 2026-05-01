use anyhow::Result;
use std::io::Write;
use std::process::Command;

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/closures.nix");

/// Our in-process closure-size computation must agree with `nix path-info
/// --closure-size`, since we dropped that flag from the load path for speed.
#[tokio::test]
async fn closure_size_matches_nix() -> Result<()> {
    let out = Command::new("nix-build")
        .args([FIXTURE, "-A", "v1", "--no-out-link"])
        .output()?;
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let root = String::from_utf8(out.stdout)?.trim().to_string();

    let graph = nix_tree::nix::query_path_info(&[root.clone()], true, &Default::default()).await?;
    let stats = nix_tree::path_stats::calculate_stats(&graph);

    let expected: u64 = String::from_utf8(
        Command::new("nix")
            .args([
                "--extra-experimental-features",
                "nix-command",
                "path-info",
                "--closure-size",
                &root,
            ])
            .output()?
            .stdout,
    )?
    .split_whitespace()
    .last()
    .unwrap()
    .parse()?;

    assert_eq!(stats[&root].closure_size, expected);
    Ok(())
}

/// https://github.com/Mic92/nix-tree-rs/issues/23
#[tokio::test]
async fn unbuilt_derivation_flag() -> Result<()> {
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
