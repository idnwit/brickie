//! `brickie design`: read and edit the design, versioned. The CLI writes as `agent`.
//!
//! Exit code: 0 ok, 2 any error (nothing written), 3 `design/stale` (the base is not the head;
//! nothing written).

mod args;
mod batch;
mod ops;
mod render;

pub use args::DesignArgs;

use super::args::Format;
use anyhow::Context;
use anyhow::Result;
use args::Cmd;
use brickie_change::{By, Design, LevelView, Op, Stale, Version, brick_path};
use serde::Serialize;
use std::io::Read;

pub fn run(args: DesignArgs) -> i32 {
    match dispatch(args) {
        Ok(()) => 0,
        Err(e) => match e.downcast_ref::<Stale>() {
            Some(s) => {
                eprintln!("{s}");
                3
            }
            None => {
                eprintln!("brickie: error: {e:#}");
                2
            }
        },
    }
}

fn print<T: Serialize>(json: bool, value: &T, text: impl FnOnce() -> String) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        print!("{}", text());
    }
    Ok(())
}

fn dispatch(a: DesignArgs) -> Result<()> {
    let json = matches!(a.format, Format::Json);
    let d = Design::at(&a.config)?;
    let (ops, w) = match a.cmd {
        Cmd::Show { since: None, level } => return show(&d, json, level.as_deref()),
        Cmd::Show { since: Some(n), .. } => {
            let log: Vec<_> = d.log()?.into_iter().filter(|e| e.v > n).collect();
            let head = d.head()?;
            return print(
                json,
                &serde_json::json!({"head": head.v, "versions": log}),
                || render::since(&head, n, &log),
            );
        }
        Cmd::Log => {
            let log = d.log()?;
            return print(json, &log, || render::log(&log));
        }
        Cmd::Diff { a: x, b } => {
            let b = match b {
                Some(b) => b,
                None => d.head()?.v,
            };
            let lines = d.diff(x, b)?;
            return print(json, &lines, || render::lines(&lines));
        }
        Cmd::Undo { v, base } => {
            let out = d.undo(base, By::Agent, v)?;
            return print(json, &out.version, || render::outcome(&out));
        }
        Cmd::Apply { w } => {
            let mut script = String::new();
            std::io::stdin().read_to_string(&mut script)?;
            (batch::ops(&d, &script)?, w)
        }
        Cmd::Edit(e) => ops::edit(&d, e)?,
    };
    write(&d, json, w, ops)
}

fn show(d: &Design, json: bool, level: Option<&str>) -> Result<()> {
    let (mut v, warnings) = d.show_checked()?;
    let since = since_agent(&d.log()?);
    if let Some(p) = level {
        v.root = within(v.root, &brick_path(p))?;
    }
    let mut value = serde_json::to_value(&v)?;
    if json && !since.is_empty() {
        value["since_agent"] = serde_json::to_value(&since)?;
    }
    if json && !warnings.is_empty() {
        value["warnings"] = serde_json::to_value(&warnings)?;
    }
    print(json, &value, || {
        render::warnings(&warnings) + &render::since_agent(&since) + &render::show(&v)
    })
}

/// The versions after the latest `agent` one (none when no agent has written).
fn since_agent(log: &[Version]) -> Vec<Version> {
    match log.iter().rposition(|e| e.by == By::Agent) {
        Some(i) => log[i + 1..].to_vec(),
        None => Vec::new(),
    }
}

/// The level owned by the composite at `path` (empty: the root level).
fn within(root: LevelView, path: &[String]) -> Result<LevelView> {
    let mut level = root;
    for name in path {
        let brick = level.bricks.into_iter().find(|b| &b.name == name);
        let brick = brick.with_context(|| format!("no brick `{name}` in {}", level.path))?;
        level = *brick
            .children
            .with_context(|| format!("brick `{name}` is not a composite: it owns no level"))?;
    }
    Ok(level)
}

fn write(d: &Design, json: bool, w: args::Write, ops: Vec<Op>) -> Result<()> {
    if w.dry_run {
        let plan = d.dry_run(w.base, ops)?;
        let head = d.head()?.v;
        return print(json, &render::plan_json(head, &plan), || {
            render::plan(head, &plan)
        });
    }
    let out = d.apply(w.base, By::Agent, w.note, ops)?;
    print(json, &out.version, || render::outcome(&out))
}
