//! `brickie interface <brick>`: a brick's declared interface (or, with `--from`, the sites of
//! one edge into it) as text or JSON, for coding agents and reviewers.

use anyhow::Result;
use brickie_change::working;
use brickie_check::check::Loaded;
use brickie_check::interface::{self, EdgeItem, Interface, Site};
use std::fmt::Write;
use std::path::Path;

/// Exit code 0, or 2 for a usage, config or cache problem.
pub fn run(config: &Path, brick: &str, from: Option<&str>, json: bool) -> Result<i32> {
    Ok(match print(config, brick, from, json) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    })
}

fn print(config: &Path, brick: &str, from: Option<&str>, json: bool) -> Result<()> {
    let Loaded {
        cfg,
        cache,
        resolved,
        ..
    } = working::load(config)?.0;
    let path = interface::parse_path(&cfg, brick)?;
    let out = match from {
        Some(a) => {
            let a = interface::parse_path(&cfg, a)?;
            let items = interface::edge_uses(&cache, &resolved, &a, &path);
            if json {
                serde_json::to_string_pretty(&items)?
            } else {
                edge_text(&a.join("/"), &path.join("/"), &items)
            }
        }
        None => {
            let i = interface::interface(&cache, &cfg, &resolved, &path);
            if json {
                serde_json::to_string_pretty(&i)?
            } else {
                text(&i)
            }
        }
    };
    println!("{}", out.trim_end());
    Ok(())
}

/// Sites printed per using brick in the text form.
const TEXT_SITES: usize = 5;

fn site(s: &Site) -> String {
    let test = if s.scope == "test" { " [test]" } else { "" };
    format!("{}:{}:{}  `{}`{test}", s.file, s.line, s.col, s.written)
}

fn text(i: &Interface) -> String {
    let mut o = format!(
        "{} \u{b7} {} items \u{b7} {} unused\n",
        i.brick,
        i.items.len(),
        i.unused()
    );
    for it in &i.items {
        let mark = if it.unused() {
            "  UNUSED".to_string()
        } else {
            format!("  {} uses", it.use_count())
        };
        let _ = writeln!(o, "\n{} {}{mark}", it.kind, it.name);
        let _ = writeln!(o, "    {}", it.signature);
        if !it.doc.is_empty() {
            let _ = writeln!(o, "    /// {}", it.summary());
        }
        let _ = writeln!(o, "    defined {}:{}", it.file, it.line);
        for g in &it.uses {
            let _ = writeln!(o, "    used by {} ({})", g.brick, g.sites.len());
            for s in g.sites.iter().take(TEXT_SITES) {
                let _ = writeln!(o, "      {}", site(s));
            }
            if g.sites.len() > TEXT_SITES {
                let more = g.sites.len() - TEXT_SITES;
                let _ = writeln!(o, "      ... {more} more (--json lists all)");
            }
        }
    }
    o
}

fn edge_text(from: &str, to: &str, items: &[EdgeItem]) -> String {
    let n: usize = items.iter().map(|i| i.sites.len()).sum();
    let mut o = format!(
        "root/{from} -> root/{to} \u{b7} {} items \u{b7} {n} sites\n",
        items.len()
    );
    for it in items {
        let def = match (&it.file, it.line) {
            (Some(f), Some(l)) => format!("  defined {f}:{l}"),
            _ => format!("  in {}", it.module),
        };
        let kind = if it.kind.is_empty() { "item" } else { &it.kind };
        let _ = writeln!(o, "\n{kind} {}{def}", it.name);
        if !it.signature.is_empty() {
            let _ = writeln!(o, "    {}", it.signature);
        }
        for s in &it.sites {
            let _ = writeln!(o, "      {} ({})", site(s), s.kind);
        }
    }
    o
}
