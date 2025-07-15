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

// Trie-like structure for efficient path storage
#[derive(Debug, Clone)]
struct Treeish {
    node: String,
    children: Vec<Treeish>,
}

impl Treeish {
    fn new(node: String) -> Self {
        Treeish {
            node,
            children: Vec::new(),
        }
    }

    fn with_children(node: String, children: Vec<Treeish>) -> Self {
        Treeish { node, children }
    }

    // Convert Treeish to paths
    fn to_paths(&self) -> Vec<Vec<String>> {
        if self.children.is_empty() {
            vec![vec![self.node.clone()]]
        } else {
            let mut paths = Vec::new();
            for child in &self.children {
                for mut path in child.to_paths() {
                    path.insert(0, self.node.clone());
                    paths.push(path);
                }
            }
            paths
        }
    }
}

/// Find all paths from roots to the target path using bottom-up approach
pub fn why_depends(graph: &StorePathGraph, target: &str) -> Vec<Vec<String>> {
    // Early exit if target is not in the graph
    if graph.get_path(target).is_none() {
        return Vec::new();
    }

    // Memoization cache
    let mut cache: HashMap<String, Option<Treeish>> = HashMap::new();

    // Bottom-up traversal to build Treeish
    fn build_treeish(
        graph: &StorePathGraph,
        node: &str,
        target: &str,
        cache: &mut HashMap<String, Option<Treeish>>,
        visited: &mut HashSet<String>,
    ) -> Option<Treeish> {
        // Check cache first
        if let Some(cached) = cache.get(node) {
            return cached.clone();
        }

        // Prevent cycles
        if !visited.insert(node.to_string()) {
            return None;
        }

        let result = if node == target {
            Some(Treeish::new(node.to_string()))
        } else if let Some(store_path) = graph.get_path(node) {
            let mut child_trees = Vec::new();

            for reference in &store_path.references {
                if let Some(tree) = build_treeish(graph, reference, target, cache, visited) {
                    child_trees.push(tree);
                }
            }

            if child_trees.is_empty() {
                None
            } else {
                Some(Treeish::with_children(node.to_string(), child_trees))
            }
        } else {
            None
        };

        visited.remove(node);
        cache.insert(node.to_string(), result.clone());
        result
    }

    // Build trees from roots
    let mut all_paths = Vec::new();
    for root in &graph.roots {
        let mut visited = HashSet::new();
        if let Some(tree) = build_treeish(graph, root, target, &mut cache, &mut visited) {
            let paths = tree.to_paths();
            all_paths.extend(paths);
        }
    }

    // Limit output (no sorting, to match Haskell implementation)
    all_paths.truncate(1000);
    all_paths
}
