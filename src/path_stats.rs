use crate::store_path::StorePathGraph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PathStats {
    pub closure_size: u64,
    pub added_size: Option<u64>, // None means not yet calculated
    pub immediate_parents: Vec<String>,
}

pub fn calculate_stats(graph: &StorePathGraph) -> HashMap<String, PathStats> {
    let mut stats = HashMap::new();

    // When using --recursive, nix already gave us the full closure
    // So we can use the closure_size field directly if available
    for path in &graph.paths {
        let closure_size = if let Some(size) = path.closure_size {
            size
        } else {
            // Fallback: calculate closure size manually if not provided
            let mut closure_cache: HashMap<String, HashSet<String>> = HashMap::new();
            let closure = calculate_closure(graph, &path.path, &mut closure_cache);
            closure
                .iter()
                .filter_map(|p| graph.get_path(p))
                .map(|p| p.nar_size)
                .sum()
        };

        let immediate_parents = graph
            .get_referrers(&path.path)
            .into_iter()
            .map(|p| p.path.clone())
            .collect();

        stats.insert(
            path.path.clone(),
            PathStats {
                closure_size,
                added_size: None, // Will be calculated on-demand
                immediate_parents,
            },
        );
    }

    // Skip added sizes calculation for now - it's too slow for large graphs
    // This will be calculated on-demand when displaying in the UI

    stats
}

fn calculate_closure(
    graph: &StorePathGraph,
    path: &str,
    cache: &mut HashMap<String, HashSet<String>>,
) -> HashSet<String> {
    if let Some(cached) = cache.get(path) {
        return cached.clone();
    }

    let mut closure = HashSet::new();
    let mut to_visit = vec![path.to_string()];

    while let Some(current) = to_visit.pop() {
        if closure.insert(current.clone()) {
            if let Some(store_path) = graph.get_path(&current) {
                for reference in &store_path.references {
                    if !closure.contains(reference) {
                        to_visit.push(reference.clone());
                    }
                }
            }
        }
    }

    cache.insert(path.to_string(), closure.clone());
    closure
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Alphabetical,
    ClosureSize,
    AddedSize,
}

impl SortOrder {
    pub fn next(&self) -> Self {
        match self {
            SortOrder::Alphabetical => SortOrder::ClosureSize,
            SortOrder::ClosureSize => SortOrder::AddedSize,
            SortOrder::AddedSize => SortOrder::Alphabetical,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Alphabetical => "name",
            SortOrder::ClosureSize => "closure size",
            SortOrder::AddedSize => "added size",
        }
    }
}

pub fn sort_paths(paths: &mut [String], stats: &HashMap<String, PathStats>, order: SortOrder) {
    paths.sort_by(|a, b| {
        let stat_a = stats.get(a);
        let stat_b = stats.get(b);

        match order {
            SortOrder::Alphabetical => a.cmp(b),
            SortOrder::ClosureSize => {
                let size_a = stat_a.map(|s| s.closure_size).unwrap_or(0);
                let size_b = stat_b.map(|s| s.closure_size).unwrap_or(0);
                size_b.cmp(&size_a)
            }
            SortOrder::AddedSize => {
                let size_a = stat_a.and_then(|s| s.added_size).unwrap_or(0);
                let size_b = stat_b.and_then(|s| s.added_size).unwrap_or(0);
                size_b.cmp(&size_a)
            }
        }
    });
}
