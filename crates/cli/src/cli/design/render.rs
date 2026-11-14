//! Text output of `brickie design`.

use brickie_change::{BrickView, DesignView, LevelView, Outcome, PlanAction, Version, ago};
use std::fmt::Write;

/// `ok: v16`, plus the code files scaffolded.
pub fn outcome(o: &Outcome) -> String {
    let mut s = match o.changed {
        true => format!("ok: {}\n", o.version.name()),
        false => format!("ok: {} (no change)\n", o.version.name()),
    };
    for f in &o.scaffold {
        let what = match f.action {
            PlanAction::Create => "created",
            PlanAction::Edit => "edited",
            PlanAction::Mkdir => "mkdir",
            PlanAction::Skip => "skipped",
        };
        let _ = writeln!(s, "  {what} {}: {}", f.path, f.note);
    }
    s
}

pub fn lines(lines: &[String]) -> String {
    lines.iter().map(|l| format!("{l}\n")).collect()
}

fn version(s: &mut String, e: &Version) {
    let note = e
        .note
        .as_deref()
        .map(|n| format!("  {n}"))
        .unwrap_or_default();
    let _ = writeln!(s, "{}  {}  {} ago{note}", e.name(), e.by.name(), ago(e.at));
    for d in &e.diff {
        let _ = writeln!(s, "    {d}");
    }
}

pub fn log(log: &[Version]) -> String {
    let mut s = String::new();
    log.iter().for_each(|e| version(&mut s, e));
    s
}

pub fn since(head: &Version, n: u64, log: &[Version]) -> String {
    let mut s = format!("design {} {}\n", head.name(), head.short());
    if log.is_empty() {
        let _ = writeln!(s, "no changes since v{n}");
    }
    log.iter().for_each(|e| version(&mut s, e));
    s
}

pub fn show(v: &DesignView) -> String {
    let short = &v.hash[..v.hash.len().min(8)];
    let mut s = format!(
        "design v{} {short} (by {}, {} ago)\n",
        v.v,
        v.by.name(),
        ago(v.at)
    );
    level(&mut s, &v.root, 0);
    if !v.todo.is_empty() {
        s.push_str("todo\n");
        for t in &v.todo {
            let _ = writeln!(s, "  - {}", t.text);
        }
    }
    s
}

fn list(name: &str, items: &[String]) -> String {
    match items.is_empty() {
        true => String::new(),
        false => format!("  {name} [{}]", items.join(", ")),
    }
}

fn level(s: &mut String, l: &LevelView, depth: usize) {
    let pad = "  ".repeat(depth);
    let _ = writeln!(
        s,
        "{pad}{} ({})  deps: default {}, cycles {}{}",
        l.path,
        l.file,
        l.default,
        l.cycles,
        list("exports", &l.exports)
    );
    for b in &l.bricks {
        brick(s, b, depth + 1);
    }
    for r in &l.rules {
        let via = list("via", &r.via);
        let kinds = list("kinds", &r.kinds);
        let reason = r
            .reason
            .as_deref()
            .map(|x| format!("  \"{x}\""))
            .unwrap_or_default();
        let todo = if r.todo {
            "  [todo: no code uses it]"
        } else {
            ""
        };
        let _ = writeln!(
            s,
            "{pad}  rule {} -> {}{via}{kinds} ({}) {}{reason}{todo}",
            r.from, r.to, r.scope, r.action
        );
    }
    for m in &l.limits {
        let metrics: Vec<String> = m.metrics.iter().map(|(k, v)| format!("{k} {v}")).collect();
        let _ = writeln!(
            s,
            "{pad}  limit {}: {} ({}, {})",
            m.bricks,
            metrics.join(", "),
            m.level,
            m.scope
        );
    }
}

fn brick(s: &mut String, b: &BrickView, depth: usize) {
    let pad = "  ".repeat(depth);
    let desc = b
        .desc
        .as_deref()
        .map(|d| format!("  \"{d}\""))
        .unwrap_or_default();
    let todo = if b.todo { "  [todo: no code yet]" } else { "" };
    let _ = writeln!(
        s,
        "{pad}brick {} {}{}{}{desc}{todo}",
        b.name,
        b.kind,
        list("members", &b.members),
        list("tags", &b.tags)
    );
    if let Some(c) = &b.children {
        level(s, c, depth + 1);
    }
}
