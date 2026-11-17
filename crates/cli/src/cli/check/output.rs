//! The three output formats of `brickie check`.

use anyhow::Result;
use brickie_change::Working;
use brickie_check::check::{Filter, Filtered, Loaded};
use brickie_check::config::level_name;
use brickie_check::diag::{self, Color};
use std::path::Path;

pub fn json(l: &Loaded, w: &Working, filter: &Filter, found: &Filtered) -> Result<()> {
    let ds = &found.shown;
    let mut out = serde_json::json!({
        "summary": {"errors": ds.len()},
        "diagnostics": ds,
        "working": w.name(),
        "todo": todo_json(l),
    });
    if filter.scoped() {
        out["outside_scope"] = serde_json::json!({"errors": found.outside});
    }
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn todo_json(l: &Loaded) -> serde_json::Value {
    let t = &l.todo;
    let names = |p: &[Vec<String>]| -> Vec<String> { p.iter().map(|p| level_name(p)).collect() };
    let links = |v: &[brickie_check::config::LinkState]| -> Vec<String> {
        v.iter()
            .map(|s| {
                format!(
                    "{} -> {} in {}",
                    s.link.from,
                    s.link.to,
                    level_name(&s.level)
                )
            })
            .collect()
    };
    serde_json::json!({
        "planned_bricks": names(&t.bricks),
        "planned_links": links(&t.links),
        "link_cycles": links(&t.cycles),
    })
}

/// The `todo:` section after the findings; nothing when there is no planned work.
fn todo_text(l: &Loaded) -> String {
    let lines = l.todo.lines();
    match lines.is_empty() {
        true => String::new(),
        false => format!(
            "todo:\n{}\n",
            lines
                .iter()
                .map(|x| format!("  {x}"))
                .collect::<Vec<_>>()
                .join("\n")
        ),
    }
}

pub fn text(l: &Loaded, w: &Working, filter: &Filter, found: &Filtered, color: Color) {
    let root = Path::new(&l.cache.workspace_root);
    println!("working: {}", w.name());
    print!("{}", diag::render_all(&found.shown, root, color.enabled()));
    print!("{}", found.footer(filter));
    print!("{}", todo_text(l));
}

pub fn short(l: &Loaded, w: &Working, filter: &Filter, found: &Filtered) {
    let root = Path::new(&l.cache.workspace_root);
    println!("working: {}", w.name());
    print!("{}", diag::render_short(&found.shown, root));
    print!("{}", found.footer(filter));
    print!("{}", todo_text(l));
}
