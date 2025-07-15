use crate::store_path::StorePathGraph;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PathStats {
    pub closure_size: u64,
    pub added_size: u64,
    pub immediate_parents: Vec<String>,
}

pub fn calculate_stats(graph: &StorePathGraph) -> HashMap<String, PathStats> {
    let mut stats = HashMap::new();
    let mut closure_cache: HashMap<String, HashSet<String>> = HashMap::new();

    for path in &graph.paths {
        let closure = calculate_closure(graph, &path.path, &mut closure_cache);
        let closure_size: u64 = closure
            .iter()
            .filter_map(|p| graph.get_path(p))
            .map(|p| p.nar_size)
            .sum();

        let immediate_parents = graph
            .get_referrers(&path.path)
            .into_iter()
            .map(|p| p.path.clone())
            .collect();

        stats.insert(
            path.path.clone(),
            PathStats {
                closure_size,
                added_size: 0,
                immediate_parents,
            },
        );
    }

    calculate_added_sizes(&mut stats, graph);

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
    closure.insert(path.to_string());

    if let Some(store_path) = graph.get_path(path) {
        for reference in &store_path.references {
            let ref_closure = calculate_closure(graph, reference, cache);
            closure.extend(ref_closure);
        }
    }

    cache.insert(path.to_string(), closure.clone());
    closure
}

fn calculate_added_sizes(stats: &mut HashMap<String, PathStats>, graph: &StorePathGraph) {
    for path in &graph.paths {
        let mut unique_closure = HashSet::new();
        unique_closure.insert(path.path.clone());

        for reference in &path.references {
            if let Some(_ref_stats) = stats.get(reference) {
                let ref_closure = calculate_closure_set(graph, reference);
                unique_closure.extend(ref_closure);
            }
        }

        let mut shared_with_siblings = HashSet::new();
        for parent in &stats.get(&path.path).unwrap().immediate_parents {
            if let Some(parent_path) = graph.get_path(parent) {
                for sibling_ref in &parent_path.references {
                    if sibling_ref != &path.path {
                        let sibling_closure = calculate_closure_set(graph, sibling_ref);
                        shared_with_siblings.extend(sibling_closure);
                    }
                }
            }
        }

        let unique_to_path: HashSet<_> = unique_closure
            .difference(&shared_with_siblings)
            .cloned()
            .collect();

        let added_size: u64 = unique_to_path
            .iter()
            .filter_map(|p| graph.get_path(p))
            .map(|p| p.nar_size)
            .sum();

        if let Some(path_stats) = stats.get_mut(&path.path) {
            path_stats.added_size = added_size;
        }
    }
}

fn calculate_closure_set(graph: &StorePathGraph, path: &str) -> HashSet<String> {
    let mut closure = HashSet::new();
    let mut to_visit = vec![path.to_string()];

    while let Some(current) = to_visit.pop() {
        if closure.insert(current.clone()) {
            if let Some(store_path) = graph.get_path(&current) {
                to_visit.extend(store_path.references.clone());
            }
        }
    }

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
                let size_a = stat_a.map(|s| s.added_size).unwrap_or(0);
                let size_b = stat_b.map(|s| s.added_size).unwrap_or(0);
                size_b.cmp(&size_a)
            }
        }
    });
}
