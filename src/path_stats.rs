use crate::store_path::StorePathGraph;
use std::collections::{HashMap, HashSet};

/// Adjacency list over dense integer ids so closure walks avoid hashing
/// 90-byte store-path strings on every edge.
pub struct IndexedGraph {
    nar_size: Vec<u64>,
    refs: Vec<Vec<u32>>,
}

impl IndexedGraph {
    pub fn new(graph: &StorePathGraph) -> Self {
        let n = graph.paths.len();
        let mut nar_size = Vec::with_capacity(n);
        let mut refs = Vec::with_capacity(n);
        for p in &graph.paths {
            nar_size.push(p.nar_size);
            refs.push(
                p.references
                    .iter()
                    .filter_map(|r| graph.index_of(r))
                    .map(|i| i as u32)
                    .collect(),
            );
        }
        Self { nar_size, refs }
    }

    /// `seen[i] == generation` marks visited; bumping `generation` resets in O(1).
    fn closure_size(
        &self,
        start: u32,
        seen: &mut [u32],
        generation: u32,
        stack: &mut Vec<u32>,
    ) -> u64 {
        stack.clear();
        stack.push(start);
        seen[start as usize] = generation;
        let mut size = 0u64;
        while let Some(i) = stack.pop() {
            size += self.nar_size[i as usize];
            for &r in &self.refs[i as usize] {
                if seen[r as usize] != generation {
                    seen[r as usize] = generation;
                    stack.push(r);
                }
            }
        }
        size
    }

    /// Walk from `roots` summing nar sizes, optionally never entering `skip`
    /// so the result is the closure that would remain if `skip` (and
    /// everything only it kept alive) were removed.
    fn closure_size_from(&self, roots: &[u32], skip: Option<u32>, seen: &mut [bool]) -> u64 {
        seen.fill(false);
        if let Some(s) = skip {
            seen[s as usize] = true;
        }
        let mut stack = Vec::with_capacity(roots.len());
        for &r in roots {
            if !seen[r as usize] {
                seen[r as usize] = true;
                stack.push(r);
            }
        }
        let mut size = 0u64;
        while let Some(i) = stack.pop() {
            size += self.nar_size[i as usize];
            for &r in &self.refs[i as usize] {
                if !seen[r as usize] {
                    seen[r as usize] = true;
                    stack.push(r);
                }
            }
        }
        size
    }
}

/// Reusable buffers + cached context closure for added-size queries from the
/// status bar, so scrolling within one parent only pays one full walk.
pub struct AddedSize {
    idx: IndexedGraph,
    seen: Vec<bool>,
    context_roots: Vec<u32>,
    context_total: u64,
}

impl AddedSize {
    pub fn new(graph: &StorePathGraph) -> Self {
        let idx = IndexedGraph::new(graph);
        let seen = vec![false; graph.paths.len()];
        Self {
            idx,
            seen,
            context_roots: Vec::new(),
            context_total: 0,
        }
    }

    /// Added sizes for every `item` relative to `context`, computed in one pass
    /// so the cached context total is reused across the batch.
    pub fn for_items(
        &mut self,
        graph: &StorePathGraph,
        items: &[String],
        context: &[String],
    ) -> HashMap<String, u64> {
        items
            .iter()
            .map(|p| (p.clone(), self.for_path(graph, p, context)))
            .collect()
    }

    pub fn for_path(&mut self, graph: &StorePathGraph, path: &str, context: &[String]) -> u64 {
        let roots: Vec<u32> = context
            .iter()
            .filter_map(|p| graph.index_of(p))
            .map(|i| i as u32)
            .collect();
        if roots != self.context_roots {
            self.context_total = self.idx.closure_size_from(&roots, None, &mut self.seen);
            self.context_roots = roots;
        }
        let Some(target) = graph.index_of(path) else {
            return 0;
        };
        let without =
            self.idx
                .closure_size_from(&self.context_roots, Some(target as u32), &mut self.seen);
        self.context_total.saturating_sub(without)
    }
}

#[derive(Debug, Clone)]
pub struct PathStats {
    pub closure_size: u64,
    pub immediate_parents: Vec<String>,
}

pub fn calculate_stats(graph: &StorePathGraph) -> HashMap<String, PathStats> {
    let mut stats = HashMap::with_capacity(graph.paths.len());
    let mut referrers = graph.build_referrers();

    let idx = IndexedGraph::new(graph);
    let mut seen = vec![0u32; graph.paths.len()];
    let mut stack = Vec::new();

    for (i, path) in graph.paths.iter().enumerate() {
        let closure_size = idx.closure_size(i as u32, &mut seen, i as u32 + 1, &mut stack);
        let immediate_parents = referrers.remove(&path.path).unwrap_or_default();

        stats.insert(
            path.path.clone(),
            PathStats {
                closure_size,
                immediate_parents,
            },
        );
    }

    stats
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

pub fn sort_paths(
    paths: &mut [String],
    graph: &StorePathGraph,
    stats: &HashMap<String, PathStats>,
    order: SortOrder,
    added: Option<&HashMap<String, u64>>,
) {
    match order {
        SortOrder::Alphabetical => {
            paths.sort_by(|a, b| {
                let na = graph.get_path(a).map(|p| p.name.as_str()).unwrap_or(a);
                let nb = graph.get_path(b).map(|p| p.name.as_str()).unwrap_or(b);
                na.cmp(nb)
            });
        }
        SortOrder::ClosureSize => {
            paths.sort_by_key(|p| std::cmp::Reverse(stats.get(p).map_or(0, |s| s.closure_size)));
        }
        SortOrder::AddedSize => {
            // Without a context (e.g. referrers pane) added size is undefined;
            // closure size is the next best stable order.
            paths.sort_by_key(|p| {
                std::cmp::Reverse(
                    added
                        .and_then(|m| m.get(p).copied())
                        .unwrap_or_else(|| stats.get(p).map_or(0, |s| s.closure_size)),
                )
            });
        }
    }
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
