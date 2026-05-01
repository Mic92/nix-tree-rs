use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

use crate::store_path::{StorePath, StorePathGraph};

#[derive(Debug, Deserialize)]
struct NixPathInfo {
    #[serde(rename = "narSize")]
    nar_size: u64,
    references: Vec<String>,
    signatures: Option<Vec<String>>,
}

#[derive(Debug, Default, Clone)]
pub struct QueryOptions {
    pub store: Option<String>,
    pub nix_options: Vec<(String, String)>,
    pub file: Option<String>,
    pub derivation: bool,
    pub impure: bool,
}

/// `--file`/`--derivation` are subcommand flags, so this builds up to and
/// including `path-info` before applying them.
fn path_info_cmd(opts: &QueryOptions) -> Command {
    let mut cmd = Command::new("nix");
    cmd.arg("--extra-experimental-features")
        .arg("nix-command flakes")
        .arg("path-info")
        .arg("--json");

    for (name, value) in &opts.nix_options {
        cmd.arg("--option").arg(name).arg(value);
    }
    if let Some(store_url) = &opts.store {
        cmd.arg("--store").arg(store_url);
    }
    if let Some(file_path) = &opts.file {
        cmd.arg("--file").arg(file_path);
    }
    if opts.derivation {
        cmd.arg("--derivation");
    }
    if opts.impure {
        cmd.arg("--impure");
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd
}

async fn run_path_info<T: serde::de::DeserializeOwned>(mut cmd: Command) -> Result<T> {
    let output = cmd.output().await.context("Failed to run nix path-info")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nix path-info failed: {stderr}");
    }
    serde_json::from_slice(&output.stdout).context("Failed to parse nix path-info JSON")
}

/// Resolve flake references and other inputs to store paths
async fn resolve_paths(paths: &[String], opts: &QueryOptions) -> Result<Vec<String>> {
    let mut cmd = path_info_cmd(opts);
    cmd.args(paths);

    // For paths not in the store nix emits `"<path>": null` with exit 0, so
    // the value side must tolerate null instead of forcing NixPathInfo.
    let map: std::collections::HashMap<String, Option<serde_json::Value>> =
        run_path_info(cmd).await?;

    let mut resolved = Vec::with_capacity(map.len());
    for (path, info) in map {
        if info.is_none() {
            anyhow::bail!("store path '{path}' is not valid (output not built?); try --derivation");
        }
        resolved.push(path);
    }
    Ok(resolved)
}

pub async fn query_path_info(
    paths: &[String],
    recursive: bool,
    opts: &QueryOptions,
) -> Result<StorePathGraph> {
    // First resolve any flake references to store paths
    let resolved_paths = resolve_paths(paths, opts).await?;

    // resolved_paths are store paths; --file would misinterpret them as attrs.
    let mut cmd = path_info_cmd(&QueryOptions {
        file: None,
        ..opts.clone()
    });
    if recursive {
        cmd.arg("--recursive");
    }
    cmd.args(&resolved_paths);

    let path_info_map: std::collections::HashMap<String, NixPathInfo> = run_path_info(cmd).await?;

    let mut graph = StorePathGraph::new();

    for (path, info) in path_info_map {
        let (hash, name) = StorePath::parse(&path)?;

        let store_path = StorePath {
            path: path.clone(),
            hash,
            name,
            nar_size: info.nar_size,
            references: info.references,
            signatures: info.signatures.unwrap_or_default(),
        };

        graph.add_path(store_path);
    }

    // Use the resolved paths as roots
    graph.roots = resolved_paths;
    graph.disambiguate_names();

    Ok(graph)
}

pub async fn get_default_roots() -> Result<Vec<String>> {
    let mut roots = Vec::new();

    let system_profile = "/nix/var/nix/profiles/system";
    if tokio::fs::metadata(system_profile).await.is_ok() {
        roots.push(system_profile.to_string());
    }

    if let Ok(user) = std::env::var("USER") {
        let user_profile = format!("/nix/var/nix/profiles/per-user/{user}/profile");
        if tokio::fs::metadata(&user_profile).await.is_ok() {
            roots.push(user_profile);
        }
    }

    if roots.is_empty() {
        anyhow::bail!("No default roots found. Please specify a path.");
    }

    Ok(roots)
}
