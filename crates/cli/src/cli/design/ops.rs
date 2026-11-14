//! Write commands to ops: names and paths resolved against the design on disk.

use super::args::*;
use anyhow::{Context, Result};
use brickie_change::*;

fn level(d: &Design, l: Option<&str>) -> Result<String> {
    d.level_file(&brick_path(l.unwrap_or("")))
}

fn kind(k: Kind) -> BrickKind {
    match k {
        Kind::Crate => BrickKind::Crate,
        Kind::Module => BrickKind::Module,
        Kind::Group => BrickKind::Group,
    }
}

/// The ops of any edit command, and its write arguments.
pub fn edit(d: &Design, e: Edit) -> Result<(Vec<Op>, Write)> {
    match e {
        Edit::Brick(c) => brick(d, c),
        Edit::Rule(c) => rule(d, c),
        Edit::Export(c) => export(d, c),
        Edit::Limit(c) => limit(d, c),
        Edit::Deps(c) => deps(d, c),
        Edit::Include(c) => include(d, c),
    }
}

/// The ops of a brick command, and its write arguments.
pub fn brick(d: &Design, c: BrickCmd) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        BrickCmd::Add {
            name,
            level: l,
            kind: k,
            f,
            w,
        } => {
            let brick = BrickSpec {
                name,
                kind: k.map(kind),
                members: f.members.unwrap_or_default(),
                tags: f.tags.unwrap_or_default(),
                path: f.path,
                desc: f.desc,
            };
            let file = level(d, l.as_deref())?;
            (vec![Op::AddBrick { file, brick }], w)
        }
        BrickCmd::Set { brick, f, w } => {
            let (file, name) = d.brick_file(&brick_path(&brick))?;
            let op = Op::SetBrick {
                file,
                name,
                desc: f.desc,
                tags: f.tags,
                members: f.members,
                path: f.path,
            };
            (vec![op], w)
        }
        BrickCmd::Rename { brick, to, w } => {
            let (file, from) = d.brick_file(&brick_path(&brick))?;
            (vec![Op::RenameBrick { file, from, to }], w)
        }
        BrickCmd::Move { brick, into, w } => {
            (vec![d.move_op(&brick_path(&brick), &brick_path(&into))?], w)
        }
        BrickCmd::Rm { brick, w } => {
            let (file, name) = d.brick_file(&brick_path(&brick))?;
            (vec![Op::RemoveBrick { file, name }], w)
        }
        BrickCmd::Group { name, children, w } => (group(d, name, &children)?, w),
        BrickCmd::Ungroup { brick, w } => {
            let (file, name) = d.brick_file(&brick_path(&brick))?;
            (vec![Op::Ungroup { file, name }], w)
        }
    })
}

/// A new composite next to the first child, and every child moved into it.
fn group(d: &Design, name: String, children: &[String]) -> Result<Vec<Op>> {
    let first = brick_path(children.first().context("name the children")?);
    let parent = first[..first.len().saturating_sub(1)].to_vec();
    let file = d.level_file(&parent)?;
    let brick = BrickSpec {
        name: name.clone(),
        kind: Some(BrickKind::Group),
        ..BrickSpec::default()
    };
    let mut ops = vec![Op::AddBrick { file, brick }];
    let mut into = parent;
    into.push(name);
    for c in children {
        ops.push(d.move_op(&brick_path(c), &into)?);
    }
    Ok(ops)
}

fn key(sel: &RuleSel) -> RuleKey {
    RuleKey {
        from: sel.from.clone(),
        to: sel.to.clone(),
        kinds: (!sel.kinds.is_empty()).then(|| sel.kinds.clone()),
        scope: sel.scope.clone(),
    }
}

pub fn rule(d: &Design, c: RuleCmd) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        RuleCmd::Add { sel, action, f, w } => {
            let rule = RuleSpec {
                from: sel.from,
                to: sel.to,
                via: f.via.unwrap_or_default(),
                kinds: sel.kinds,
                scope: sel.scope,
                action,
                reason: f.reason,
                help: f.fix,
            };
            let file = level(d, sel.level.as_deref())?;
            (vec![Op::AddRule { file, rule }], w)
        }
        RuleCmd::Set { sel, action, f, w } => {
            let key = key(&sel);
            let file = d.rule_file(&level(d, sel.level.as_deref())?, &key)?;
            let set = RulePatch {
                action,
                reason: f.reason,
                via: f.via,
                help: f.fix,
            };
            (vec![Op::SetRule { file, key, set }], w)
        }
        RuleCmd::Rm { sel, w } => {
            let key = key(&sel);
            let file = d.rule_file(&level(d, sel.level.as_deref())?, &key)?;
            (vec![Op::RemoveRule { file, key }], w)
        }
    })
}

pub fn export(d: &Design, c: ExportCmd) -> Result<(Vec<Op>, Write)> {
    let (brick, scope, w) = match c {
        ExportCmd::Set { brick, scope, w } => (brick, Some(scope), w),
        ExportCmd::Rm { brick, w } => (brick, None, w),
    };
    let (file, name) = d.brick_file(&brick_path(&brick))?;
    let mut exports = d.exports(&file)?;
    exports.retain(|e| e.brick != name);
    if let Some(scope) = scope {
        exports.push(ExportSpec { brick: name, scope });
    }
    Ok((vec![Op::SetExports { file, exports }], w))
}

fn spec(bricks: Option<String>, l: &Limits) -> ComplexitySpec {
    ComplexitySpec {
        bricks,
        cognitive: l.cognitive,
        cyclomatic: l.cyclomatic,
        nesting: l.nesting,
        paths: l.paths,
        params: l.params,
        lines: l.lines,
        level: l.level.clone(),
        scope: l.scope.clone(),
        help: None,
    }
}

pub fn limit(d: &Design, c: LimitCmd) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        LimitCmd::Add { bricks, l, w } => {
            let file = level(d, l.within.as_deref())?;
            let rule = spec(bricks, &l);
            (vec![Op::AddComplexity { file, rule }], w)
        }
        LimitCmd::Set { bricks, l, w } => {
            let file = level(d, l.within.as_deref())?;
            let set = spec(None, &l);
            (vec![Op::SetComplexity { file, bricks, set }], w)
        }
        LimitCmd::Rm {
            bricks,
            level: l,
            w,
        } => {
            let file = level(d, l.as_deref())?;
            (vec![Op::RemoveComplexity { file, bricks }], w)
        }
    })
}

pub fn deps(d: &Design, c: DepsCmd) -> Result<(Vec<Op>, Write)> {
    let DepsCmd::Set {
        default,
        cycles,
        level: l,
        w,
    } = c;
    let file = level(d, l.as_deref())?;
    Ok((
        vec![Op::SetDeps {
            file,
            default,
            cycles,
        }],
        w,
    ))
}

pub fn include(d: &Design, c: IncludeCmd) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        IncludeCmd::Add { path, level: l, w } => {
            let file = level(d, l.as_deref())?;
            (vec![Op::AddInclude { file, path }], w)
        }
        IncludeCmd::Rm { path, level: l, w } => {
            let file = level(d, l.as_deref())?;
            (vec![Op::RemoveInclude { file, path }], w)
        }
    })
}
