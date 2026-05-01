//! End-to-end tests that drive the compiled `nix-tree` binary against a tiny
//! synthetic closure (see tests/fixtures/closures.nix). Hermetic: no nixpkgs.

use anyhow::{Context, Result};
use std::process::Command;
use std::sync::OnceLock;

const BIN: &str = env!("CARGO_BIN_EXE_nix-tree");
const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/closures.nix");

struct Fixture {
    v1: String,
    v2: String,
}

fn fixture() -> &'static Fixture {
    static F: OnceLock<Fixture> = OnceLock::new();
    F.get_or_init(|| {
        let out = Command::new("nix-build")
            .args([FIXTURE, "-A", "v1", "-A", "v2", "--no-out-link"])
            .output()
            .expect("spawn nix-build");
        assert!(
            out.status.success(),
            "nix-build failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let paths: Vec<String> = String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(paths.len(), 2, "expected two out paths");
        Fixture {
            v1: paths[0].clone(),
            v2: paths[1].clone(),
        }
    })
}

fn run(args: &[&str]) -> Result<String> {
    let out = Command::new(BIN).args(args).output()?;
    if !out.status.success() {
        anyhow::bail!(
            "{BIN} {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    String::from_utf8(out.stdout).context("non-utf8 stdout")
}

#[test]
fn dot_output_shape() -> Result<()> {
    let f = fixture();
    let dot = run(&["--dot", &f.v1])?;

    assert!(dot.starts_with("strict digraph {\n"), "got: {dot}");
    assert!(dot.trim_end().ends_with('}'));
    assert!(dot.contains(r#""ntfx-app-1.0" -> "ntfx-liba-1.0""#));
    assert!(dot.contains(r#""ntfx-app-1.0" -> "ntfx-libb-1.0""#));
    assert!(dot.contains(r#""ntfx-liba-1.0" -> "ntfx-libc-1.0""#));
    assert!(dot.contains(r#""ntfx-libb-1.0" -> "ntfx-libc-1.0""#));
    // No edge that doesn't exist in the fixture.
    assert!(!dot.contains("libd"));
    // No self-reference noise.
    assert!(!dot.contains(r#""ntfx-libc-1.0" -> "ntfx-libc-1.0""#));

    Ok(())
}

#[test]
fn diff_output() -> Result<()> {
    let f = fixture();
    let diff = run(&["--diff", &f.v1, &f.v2])?;
    let lines: Vec<&str> = diff.lines().collect();

    // libb removed, libd added, app version bumped.
    let libb = lines
        .iter()
        .find(|l| l.contains("ntfx-libb"))
        .expect("libb row");
    assert!(libb.contains("→ ∅"), "libb should be removed: {libb}");

    let libd = lines
        .iter()
        .find(|l| l.contains("ntfx-libd"))
        .expect("libd row");
    assert!(libd.contains("∅ →"), "libd should be added: {libd}");

    let app = lines
        .iter()
        .find(|l| l.contains("ntfx-app"))
        .expect("app row");
    assert!(app.contains("1.0 → 2.0"), "app version bump: {app}");

    // liba/libc are identical paths in both → must be filtered.
    assert!(!diff.contains("ntfx-liba"));
    assert!(!diff.contains("ntfx-libc"));

    // Summary line with totals.
    let summary = lines.last().unwrap();
    assert!(
        summary.contains("paths →") && summary.contains('('),
        "summary: {summary}"
    );

    // stdout is a pipe → no ANSI escapes.
    assert!(!diff.contains('\x1b'));

    Ok(())
}

#[test]
fn diff_requires_two_args() {
    let out = Command::new(BIN).args(["--diff", "/x"]).output().unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("exactly two"));
}
