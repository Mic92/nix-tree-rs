use crate::store_path::StorePathGraph;
use std::collections::{BTreeSet, HashMap};
use std::io::{self, IsTerminal, Write};

#[derive(Clone, Copy)]
struct Ansi(bool);
impl Ansi {
    const RED: &'static str = "\x1b[31;1m";
    const GREEN: &'static str = "\x1b[32;1m";
    const DIM: &'static str = "\x1b[2m";
    const RESET: &'static str = "\x1b[0m";
    fn paint(self, code: &str, s: &str) -> String {
        if self.0 {
            format!("{code}{s}{}", Self::RESET)
        } else {
            s.to_string()
        }
    }
}

/// Derivation outputs (-dev, -man, -lib, ...) end up either on the version
/// (hello-2.12-man -> version "2.12-man") or, for unversioned paths, on the
/// pname. Fold them away so all outputs of one derivation group together.
fn strip_output_suffix<'a>(pname: &'a str, version: &'a str) -> (&'a str, &'a str) {
    fn strip(s: &str) -> Option<&str> {
        let i = s.rfind('-')?;
        let suffix = &s[i + 1..];
        let known = !suffix.is_empty()
            && (suffix.bytes().all(|b| b.is_ascii_lowercase())
                || suffix == "lib32"
                || suffix == "lib64");
        known.then_some(&s[..i])
    }
    if !version.is_empty() {
        (pname, strip(version).unwrap_or(version))
    } else {
        (strip(pname).unwrap_or(pname), version)
    }
}

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
        let (pname, version) = strip_output_suffix(pname, version);
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

    // Match nix store diff-closures: hide entries that only changed hash
    // (propagated rebuilds) unless their size moved noticeably.
    const THRESHOLD: u64 = 8 * 1024;
    rows.retain(|r| {
        let bv = r.before.as_ref().map(|s| &s.versions);
        let av = r.after.as_ref().map(|s| &s.versions);
        bv != av || r.delta.unsigned_abs() >= THRESHOLD
    });

    rows.sort_by_key(|r| std::cmp::Reverse(r.delta.unsigned_abs()));

    let ansi = Ansi(io::stdout().is_terminal());
    let name_w = rows.iter().map(|r| r.pname.len()).max().unwrap_or(0);

    for r in &rows {
        let bv = r.before.as_ref().map(fmt_versions);
        let av = r.after.as_ref().map(fmt_versions);
        let change = match (bv, av) {
            (None, Some(v)) => ansi.paint(Ansi::GREEN, &format!("∅ → {v}")),
            (Some(v), None) => ansi.paint(Ansi::RED, &format!("{v} → ∅")),
            (Some(b), Some(a)) if b == a => ansi.paint(Ansi::DIM, "(rebuilt)"),
            (Some(b), Some(a)) => format!("{b} → {a}"),
            (None, None) => unreachable!(),
        };
        // Pad before colouring so escape bytes don't skew the width.
        let delta = format!("{:>12}", fmt_delta(r.delta));
        let delta = match r.delta.signum() {
            1 => ansi.paint(Ansi::RED, &delta),
            -1 => ansi.paint(Ansi::GREEN, &delta),
            _ => ansi.paint(Ansi::DIM, &delta),
        };
        writeln!(out, "{delta}  {:name_w$}  {change}", r.pname)?;
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

    use super::strip_output_suffix;

    #[test]
    fn output_suffix_stripping() {
        assert_eq!(strip_output_suffix("hello", "2.12-man"), ("hello", "2.12"));
        assert_eq!(strip_output_suffix("jq", "1.8.1-bin"), ("jq", "1.8.1"));
        assert_eq!(
            strip_output_suffix("hm-session-vars.sh", ""),
            ("hm-session-vars.sh", "")
        );
        assert_eq!(strip_output_suffix("ncurses", "6.6"), ("ncurses", "6.6"));
        assert_eq!(
            strip_output_suffix("glibc", "2.40-66"),
            ("glibc", "2.40-66")
        );
        assert_eq!(strip_output_suffix("git-man", ""), ("git", ""));
    }

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
