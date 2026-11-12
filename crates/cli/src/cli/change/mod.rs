//! `brickie change`: the open design change for the coding agent (and the human on the CLI).
//!
//! Exit code: 0 ok, 1 publish refused, 2 any other error (no change open, invalid op, ...).

mod render;

use super::args::{ChangeArgs, ChangeCmd, Format, Who};
use anyhow::{Context, Result};
use brickie_change::{By, Change, Op, Verdict};
use serde::Serialize;
use std::io::Read;

pub fn run(args: ChangeArgs) -> i32 {
    match dispatch(args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("brickie: error: {e:#}");
            2
        }
    }
}

fn by(who: Who) -> By {
    match who {
        Who::Human => By::Human,
        Who::Agent => By::Agent,
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

fn dispatch(a: ChangeArgs) -> Result<i32> {
    let json = matches!(a.format, Format::Json);
    let Some(action) = a.action else {
        return show(&a, json);
    };
    if let ChangeCmd::Open { intent } = &action {
        let c = Change::open(&a.config, intent)?;
        let s = c.status()?;
        print(json, &s, || format!("opened change: {}\n", s.intent))?;
        return Ok(0);
    }
    let c = Change::load(&a.config)?;
    match action {
        ChangeCmd::Open { .. } => unreachable!("handled above"),
        ChangeCmd::Propose { json: _, note } => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            let op: Op = serde_json::from_str(&input).context("parse the op JSON on stdin")?;
            let r = c.record(by(a.who), op, note)?;
            print(json, &r, || render::record(&r))?;
        }
        ChangeCmd::Accept { seq, note } => review(&c, seq, Verdict::Accept, note, json)?,
        ChangeCmd::Reject { seq, note } => review(&c, seq, Verdict::Reject, note, json)?,
        ChangeCmd::Retract { seq } => {
            let r = c.retract(seq)?;
            print(json, &r, || {
                format!("#{} retracted op {}\n", r.seq, r.target)
            })?;
        }
        ChangeCmd::Redo => {
            let r = c.redo()?;
            print(json, &r, || {
                format!("#{} restored op {}\n", r.seq, r.target)
            })?;
        }
        ChangeCmd::Reapply { seq } => {
            let r = c.reapply(seq)?;
            print(json, &r, || render::record(&r))?;
        }
        ChangeCmd::Publish { dry_run: true } => {
            let plan = c.publish_plan()?;
            print(json, &plan, || render::plan(&plan))?;
        }
        ChangeCmd::Publish { dry_run: false } => {
            let blockers = c.publish_plan()?.blockers;
            if !blockers.is_empty() {
                eprintln!("brickie: publish refused:\n  {}", blockers.join("\n  "));
                return Ok(1);
            }
            let plan = c.publish()?;
            print(json, &plan, || {
                format!("published\n{}", render::plan(&plan))
            })?;
        }
        ChangeCmd::Discard => {
            c.discard()?;
            print(json, &serde_json::json!({"discarded": true}), || {
                "change discarded\n".into()
            })?;
        }
    }
    Ok(0)
}

fn review(c: &Change, seq: u64, v: Verdict, note: Option<String>, json: bool) -> Result<()> {
    let r = c.review(By::Human, seq, v, note)?;
    print(json, &r, || {
        format!("#{} {:?} op {}\n", r.seq, r.verdict, r.target).to_lowercase()
    })
}

#[derive(Serialize)]
struct Show {
    status: brickie_change::Status,
    diff: Vec<brickie_change::DiffEntry>,
    new: brickie_change::Since,
}

fn show(a: &ChangeArgs, json: bool) -> Result<i32> {
    let c = Change::load(&a.config)?;
    if a.todo {
        let tasks = c.todo()?;
        print(json, &tasks, || render::todo(&tasks))?;
        return Ok(0);
    }
    let s = Show {
        status: c.status()?,
        diff: c.diff()?,
        new: c.since(by(a.who), a.since)?,
    };
    print(json, &s, || render::show(&s.status, &s.diff, &s.new))?;
    Ok(0)
}
