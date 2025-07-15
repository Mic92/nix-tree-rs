use anyhow::{Context, Result};
use serde::Deserialize;
use std::process::Stdio;
use tokio::process::Command;

use crate::store_path::{StorePath, StorePathGraph};

#[derive(Debug, Deserialize)]
struct NixPathInfo {
    #[serde(rename = "narHash")]
    #[allow(dead_code)]
    nar_hash: Option<String>,
    #[serde(rename = "narSize")]
    nar_size: u64,
    references: Vec<String>,
    signatures: Option<Vec<String>>,
    #[serde(rename = "closureSize")]
    closure_size: Option<u64>,
}

pub async fn query_path_info(
    paths: &[String],
    recursive: bool,
    store: Option<&str>,
) -> Result<StorePathGraph> {
    let mut cmd = Command::new("nix");
    cmd.arg("path-info")
        .arg("--json")
        .args(paths)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if recursive {
        cmd.arg("--recursive");
    }

    if let Some(store_url) = store {
        cmd.arg("--store").arg(store_url);
    }

    let output = cmd
        .output()
        .await
        .context("Failed to execute nix path-info")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("nix path-info failed: {}", stderr);
    }

    let json_str = String::from_utf8(output.stdout).context("Invalid UTF-8 in nix output")?;

    let path_info_map: std::collections::HashMap<String, NixPathInfo> =
        serde_json::from_str(&json_str).context("Failed to parse nix path-info JSON")?;

    let mut graph = StorePathGraph::new();

    for (path, info) in path_info_map {
        let (hash, name) = StorePath::parse(&path)?;

        let store_path = StorePath {
            path: path.clone(),
            hash,
            name,
            nar_size: info.nar_size,
            closure_size: info.closure_size,
            references: info.references,
            signatures: info.signatures.unwrap_or_default(),
        };

        graph.add_path(store_path);
    }

    graph.roots = paths.to_vec();
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

