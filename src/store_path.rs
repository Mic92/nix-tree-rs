use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct StorePath {
    pub path: String,
    pub hash: String,
    pub name: String,
    pub nar_size: u64,
    pub closure_size: Option<u64>,
    pub references: Vec<String>,
    pub signatures: Vec<String>,
}

impl StorePath {
    pub fn parse(path: &str) -> Result<(String, String)> {
        let path = path.trim();

        if !path.starts_with("/nix/store/") {
            bail!("Invalid store path: {}", path);
        }

        let without_prefix = path.strip_prefix("/nix/store/").unwrap();
        let parts: Vec<&str> = without_prefix.splitn(2, '-').collect();

        if parts.len() != 2 {
            bail!("Invalid store path format: {}", path);
        }

        let hash = parts[0].to_string();
        let name = parts[1].to_string();

        if hash.len() != 32 {
            bail!("Invalid store path hash length: {}", hash);
        }

        Ok((hash, name))
    }

    pub fn short_name(&self) -> &str {
        &self.name
    }

    pub fn is_signed(&self) -> bool {
        !self.signatures.is_empty()
    }
}

impl fmt::Display for StorePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

#[derive(Debug, Clone)]
pub struct StorePathGraph {
    pub paths: Vec<StorePath>,
    pub roots: Vec<String>,
}

impl Default for StorePathGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl StorePathGraph {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            roots: Vec::new(),
        }
    }

    pub fn add_path(&mut self, path: StorePath) {
        self.paths.push(path);
    }

    pub fn get_path(&self, path: &str) -> Option<&StorePath> {
        self.paths.iter().find(|p| p.path == path)
    }

    pub fn get_references(&self, path: &str) -> Vec<&StorePath> {
        if let Some(store_path) = self.get_path(path) {
            store_path
                .references
                .iter()
                .filter(|ref_path| ref_path != &path) // Filter out self-references
                .filter_map(|ref_path| self.get_path(ref_path))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_referrers(&self, path: &str) -> Vec<&StorePath> {
        self.paths
            .iter()
            .filter(|p| p.path != path && p.references.contains(&path.to_string()))
            .collect()
    }

    pub fn disambiguate_names(&mut self) {
        let mut name_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for path in &self.paths {
            *name_counts.entry(path.name.clone()).or_insert(0) += 1;
        }

        for path in &mut self.paths {
            if name_counts.get(&path.name).copied().unwrap_or(0) > 1 {
                path.name = format!("{}-{}", &path.hash[..8], path.name);
            }
        }
    }
}
