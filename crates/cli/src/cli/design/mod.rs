//! `brickie design`: read and edit the design, versioned. The CLI writes as `agent`.
//!
//! Exit code: 0 ok, 2 any error (nothing written), 3 `design/stale` (the base is not the head;
//! nothing written).

mod args;
mod ops;
mod render;

pub use args::DesignArgs;

use super::args::Format;
use anyhow::Result;
use args::Cmd;
use brickie_change::{By, Design, Op, Stale};
use serde::Serialize;

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
        Cmd::Show { since: None } => {
            let v = d.show()?;
            return print(json, &v, || render::show(&v));
        }
        Cmd::Show { since: Some(n) } => {
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
        Cmd::Undo { v, w } => {
            let out = d.undo(w.base, By::Agent, v)?;
            return print(json, &out.version, || render::outcome(&out));
        }
        Cmd::Brick(c) => ops::brick(&d, c)?,
        Cmd::Rule(c) => ops::rule(&d, c)?,
        Cmd::Export(c) => ops::export(&d, c)?,
        Cmd::Limit(c) => ops::limit(&d, c)?,
        Cmd::Deps(c) => ops::deps(&d, c)?,
        Cmd::Include(c) => ops::include(&d, c)?,
    };
    write(&d, json, w, ops)
}

fn write(d: &Design, json: bool, w: args::Write, ops: Vec<Op>) -> Result<()> {
    let out = d.apply(w.base, By::Agent, w.note, ops)?;
    print(json, &out.version, || render::outcome(&out))
}
