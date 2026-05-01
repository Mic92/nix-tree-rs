use crate::store_path::StorePathGraph;
use std::io::{self, Write};

/// Emit the reference graph in Graphviz dot format, matching the output shape
/// of the Haskell nix-tree so existing tooling/pipelines keep working.
pub fn write(graph: &StorePathGraph, out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "strict digraph {{")?;
    for p in &graph.paths {
        for r in &p.references {
            if r == &p.path {
                continue;
            }
            if let Some(dst) = graph.get_path(r) {
                writeln!(out, "  {} -> {} [];", quote(&p.name), quote(&dst.name))?;
            }
        }
    }
    writeln!(out, "}}")
}

fn quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
