use crate::store_path::StorePathGraph;
use std::collections::{BTreeSet, HashMap};
use std::io::{self, Write};

/// Mirrors nix's builtins.parseDrvName: the version is the suffix starting at
/// the first `-` that is followed by a digit; everything before is the pname.
fn parse_drv_name(name: &str) -> (&str, &str) {
    let bytes = name.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'-' && bytes[i + 1].is_ascii_digit() {
            return (&name[..i], &name[i + 1..]);
        }
        i += 1;
    }
    (name, "")
}

#[derive(Default)]
struct Side {
    paths: BTreeSet<String>,
    versions: BTreeSet<String>,
    nar_size: u64,
}

fn group_by_pname(graph: &StorePathGraph) -> (HashMap<String, Side>, u64) {
    let mut groups: HashMap<String, Side> = HashMap::new();
    let mut total = 0u64;
    for p in &graph.paths {
        total += p.nar_size;
        // disambiguate_names() may have prefixed a hash slice; derive the
        // original name from the path so duplicates group together.
        let raw_name = p
            .path
            .get(11 + p.hash.len() + 1..)
            .unwrap_or(p.name.as_str());
        let (pname, version) = parse_drv_name(raw_name);
        let entry = groups.entry(pname.to_string()).or_default();
        entry.paths.insert(p.path.clone());
        if !version.is_empty() {
            entry.versions.insert(version.to_string());
        }
        entry.nar_size += p.nar_size;
    }
    (groups, total)
}

struct Row {
    pname: String,
    before: Option<Side>,
    after: Option<Side>,
    delta: i64,
}

/// Print a closure diff between `old` and `new` in the spirit of
/// `nix store diff-closures`, but sorted by absolute size impact and with a
/// totals line so the biggest contributors to closure growth come first.
pub fn write(old: &StorePathGraph, new: &StorePathGraph, out: &mut impl Write) -> io::Result<()> {
    let (mut a, total_a) = group_by_pname(old);
    let (b, total_b) = group_by_pname(new);

    let mut rows = Vec::new();
    for (pname, after) in b {
        let before = a.remove(&pname);
        if let Some(before) = &before {
            if before.paths == after.paths {
                continue;
            }
        }
        let delta = after.nar_size as i64 - before.as_ref().map_or(0, |s| s.nar_size) as i64;
        rows.push(Row {
            pname,
            before,
            after: Some(after),
            delta,
        });
    }
    for (pname, before) in a {
        rows.push(Row {
            delta: -(before.nar_size as i64),
            pname,
            before: Some(before),
            after: None,
        });
    }

    rows.sort_by_key(|r| std::cmp::Reverse(r.delta.unsigned_abs()));

    for r in &rows {
        let bv = r.before.as_ref().map(fmt_versions);
        let av = r.after.as_ref().map(fmt_versions);
        let change = match (bv, av) {
            (None, Some(v)) => format!("∅ → {v}"),
            (Some(v), None) => format!("{v} → ∅"),
            (Some(b), Some(a)) if b == a => "(rebuilt)".to_string(),
            (Some(b), Some(a)) => format!("{b} → {a}"),
            (None, None) => unreachable!(),
        };
        writeln!(out, "{:>12}  {}: {}", fmt_delta(r.delta), r.pname, change)?;
    }

    writeln!(out)?;
    writeln!(
        out,
        "{} paths → {} paths, {} → {} ({})",
        old.paths.len(),
        new.paths.len(),
        bytesize::ByteSize(total_a),
        bytesize::ByteSize(total_b),
        fmt_delta(total_b as i64 - total_a as i64),
    )
}

fn fmt_versions(s: &Side) -> String {
    if s.versions.is_empty() {
        "ε".to_string()
    } else {
        s.versions
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn fmt_delta(d: i64) -> String {
    let sign = if d >= 0 { "+" } else { "-" };
    format!("{sign}{}", bytesize::ByteSize(d.unsigned_abs()))
}

#[cfg(test)]
mod tests {
    use super::parse_drv_name;

    #[test]
    fn drv_name_parsing() {
        assert_eq!(parse_drv_name("hello-2.12.1"), ("hello", "2.12.1"));
        assert_eq!(
            parse_drv_name("python3.13-foo-1.0"),
            ("python3.13-foo", "1.0")
        );
        assert_eq!(parse_drv_name("glibc-2.40-66"), ("glibc", "2.40-66"));
        assert_eq!(parse_drv_name("source"), ("source", ""));
        assert_eq!(
            parse_drv_name("nixos-system-eve-26.05pre-git"),
            ("nixos-system-eve", "26.05pre-git")
        );
    }
}
