//! `design apply`: one op per stdin line, parsed with the same clap definitions as the write
//! subcommands.

use super::args::Line;
use super::ops;
use anyhow::{Result, bail};
use brickie_change::{Design, Op};
use clap::Parser;

/// Split a line into words: whitespace separates, `'...'` is literal, `"..."` allows `\"` and
/// `\\`, and a backslash outside quotes escapes the next character.
pub fn words(line: &str) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut live = false;
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_whitespace() => {
                if live {
                    out.push(std::mem::take(&mut cur));
                    live = false;
                }
            }
            '\'' => {
                live = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(c) => cur.push(c),
                        None => bail!("unterminated quote"),
                    }
                }
            }
            '"' => {
                live = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(e @ ('"' | '\\')) => cur.push(e),
                            Some(e) => {
                                cur.push('\\');
                                cur.push(e);
                            }
                            None => bail!("unterminated quote"),
                        },
                        Some(c) => cur.push(c),
                        None => bail!("unterminated quote"),
                    }
                }
            }
            '\\' => {
                live = true;
                match chars.next() {
                    Some(e) => cur.push(e),
                    None => bail!("trailing backslash"),
                }
            }
            c => {
                live = true;
                cur.push(c);
            }
        }
    }
    if live {
        out.push(cur);
    }
    Ok(out)
}

/// The ops of one line.
fn parse(d: &Design, line: &str) -> Result<Vec<Op>> {
    let mut args = words(line)?;
    // The write arguments are required by the subcommands; a line must not carry its own.
    args.extend(["--base".into(), "0".into()]);
    let l = Line::try_parse_from(args).map_err(|e| {
        let text = e.to_string();
        let first = text.lines().next().unwrap_or("invalid line");
        anyhow::anyhow!("{}", first.trim_start_matches("error: "))
    })?;
    let (ops, w) = ops::edit(d, l.edit)?;
    if w.base != 0 || w.note.is_some() || w.dry_run {
        bail!("--base, --note and --dry-run belong to the `apply` command, not to a line");
    }
    Ok(ops)
}

/// Every op of the script; the first bad line stops it, named by its number.
pub fn ops(d: &Design, script: &str) -> Result<Vec<Op>> {
    let mut all = Vec::new();
    for (i, raw) in script.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let n = i + 1;
        match parse(d, line) {
            Ok(ops) => all.extend(ops),
            Err(e) => bail!("line {n}: {e:#}; nothing was written"),
        }
    }
    if all.is_empty() {
        bail!("no ops on stdin");
    }
    Ok(all)
}
