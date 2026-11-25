//! Write commands to ops: names and paths resolved against the design on disk.

use super::args::*;
use anyhow::{Context, Result};
use brickie_change::*;
use std::collections::BTreeSet;

/// Names added by earlier lines of one `design apply` script, so later lines may reference them.
#[derive(Default)]
pub struct Pending {
    pub groups: BTreeSet<(String, String)>,
}

impl Pending {
    pub fn record(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::AddGroup { file, group } => {
                    self.groups.insert((file.clone(), group.name.clone()));
                }
                Op::RenameGroup {
                    file,
                    name,
                    new_name,
                } => {
                    self.groups.remove(&(file.clone(), name.clone()));
                    self.groups.insert((file.clone(), new_name.clone()));
                }
                Op::RemoveGroup { file, name } => {
                    self.groups.remove(&(file.clone(), name.clone()));
                }
                _ => {}
            }
        }
    }
}

fn level(d: &Design, l: Option<&str>) -> Result<String> {
    d.level_file(&brick_path(l.unwrap_or("")))
}

fn kind(k: Kind) -> BrickKind {
    match k {
        Kind::Crate => BrickKind::Crate,
        Kind::Module => BrickKind::Module,
        Kind::Composite => BrickKind::Composite,
    }
}

fn clean_list(v: Vec<String>) -> Vec<String> {
    v.into_iter().filter(|s| !s.is_empty()).collect()
}

/// The ops of any edit command, and its write arguments.
pub fn edit(d: &Design, e: Edit) -> Result<(Vec<Op>, Write)> {
    edit_with(d, e, &Pending::default())
}

/// [`edit`], also resolving forward references to groups added on earlier lines of `apply`.
pub fn edit_with(d: &Design, e: Edit, pending: &Pending) -> Result<(Vec<Op>, Write)> {
    match e {
        Edit::Brick(c) => brick(d, c),
        Edit::Group(c) => group(d, c, pending),
        Edit::Rule(c) => rule(d, c, false),
        Edit::Link(c) => rule(d, c, true),
        Edit::Export(c) => export(d, c, pending),
        Edit::Limit(c) => limit(d, c),
        Edit::Include(c) => include(d, c),
    }
}

fn resolve_pending(
    d: &Design,
    path: &[String],
    pending: &Pending,
    group_only: bool,
) -> Result<(String, String)> {
    let found = if group_only {
        d.group_file(path)
    } else {
        d.unit_file(path)
    };
    match found {
        Ok(found) => Ok(found),
        Err(err) => {
            if let Some((name, parent)) = path.split_last()
                && let Ok(file) = d.level_file(parent)
                && pending.groups.contains(&(file.clone(), name.clone()))
            {
                return Ok((file, name.clone()));
            }
            Err(err)
        }
    }
}

/// The ops of a brick command, and its write arguments.
pub fn brick(d: &Design, c: BrickCmd) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        BrickCmd::Add {
            name,
            scaffold,
            level: l,
            kind: k,
            f,
            w,
        } => {
            let brick = BrickSpec {
                name,
                kind: k.map(kind),
                members: f.members.unwrap_or_default(),
                tags: f.tags.map(clean_list).unwrap_or_default(),
                path: f.path,
                desc: f.desc,
                group: f.group,
            };
            let file = level(d, l.as_deref())?;
            (vec![Op::AddBrick { file, brick }], Write { scaffold, ..w })
        }
        BrickCmd::Set { brick, f, w } => {
            let (file, name) = d.brick_file(&brick_path(&brick))?;
            let op = Op::SetBrick {
                file,
                name,
                desc: f.desc,
                tags: f.tags.map(clean_list),
                members: f.members,
                path: f.path,
                independent: f.independent,
                group: f.group,
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
        BrickCmd::Nest { name, children, w } => (nest(d, name, &children)?, w),
        BrickCmd::Unnest { brick, w } => {
            let (file, name) = d.brick_file(&brick_path(&brick))?;
            (vec![Op::Unnest { file, name }], w)
        }
    })
}

/// A new composite next to the first child, and every child moved into it.
fn nest(d: &Design, name: String, children: &[String]) -> Result<Vec<Op>> {
    let first = brick_path(children.first().context("name the children")?);
    let parent = first[..first.len().saturating_sub(1)].to_vec();
    let file = d.level_file(&parent)?;
    let brick = BrickSpec {
        name: name.clone(),
        kind: Some(BrickKind::Composite),
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

/// The ops of a peer group command, and its write arguments.
pub fn group(d: &Design, c: GroupCmd, pending: &Pending) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        GroupCmd::Add {
            name,
            level: l,
            tags,
            desc,
            w,
        } => {
            let file = level(d, l.as_deref())?;
            let group = GroupSpec {
                name,
                tags: tags.map(clean_list).unwrap_or_default(),
                desc,
            };
            (vec![Op::AddGroup { file, group }], w)
        }
        GroupCmd::Set {
            group,
            tags,
            desc,
            w,
        } => {
            let (file, name) = resolve_pending(d, &brick_path(&group), pending, true)?;
            let op = Op::SetGroup {
                file,
                name,
                tags: tags.map(clean_list),
                desc,
            };
            (vec![op], w)
        }
        GroupCmd::Rename { group, to, w } => {
            let (file, name) = resolve_pending(d, &brick_path(&group), pending, true)?;
            (
                vec![Op::RenameGroup {
                    file,
                    name,
                    new_name: to,
                }],
                w,
            )
        }
        GroupCmd::Rm { group, w } => {
            let (file, name) = resolve_pending(d, &brick_path(&group), pending, true)?;
            (vec![Op::RemoveGroup { file, name }], w)
        }
    })
}

fn key(sel: &RuleSel) -> RuleKey {
    RuleKey {
        from: sel.from.clone(),
        to: sel.to.clone(),
        kinds: (!sel.kinds.is_empty()).then(|| sel.kinds.clone()),
        scope: sel.scope.clone(),
    }
}

/// The ops of a rule command, or with `link` of a link command.
pub fn rule(d: &Design, c: RuleCmd, link: bool) -> Result<(Vec<Op>, Write)> {
    Ok(match c {
        RuleCmd::Add { sel, reason, w } => {
            let spec = RuleSpec {
                from: sel.from,
                to: sel.to,
                kinds: sel.kinds,
                scope: sel.scope,
                reason,
            };
            let file = level(d, sel.level.as_deref())?;
            let op = match link {
                true => Op::AddLink { file, link: spec },
                false => Op::AddRule { file, rule: spec },
            };
            (vec![op], w)
        }
        RuleCmd::Set { sel, reason, w } => {
            let key = key(&sel);
            let file = d.rule_file(&level(d, sel.level.as_deref())?, link, &key)?;
            let set = RulePatch { reason };
            let op = match link {
                true => Op::SetLink { file, key, set },
                false => Op::SetRule { file, key, set },
            };
            (vec![op], w)
        }
        RuleCmd::Rm { sel, w } => {
            let key = key(&sel);
            let file = d.rule_file(&level(d, sel.level.as_deref())?, link, &key)?;
            let op = match link {
                true => Op::RemoveLink { file, key },
                false => Op::RemoveRule { file, key },
            };
            (vec![op], w)
        }
    })
}

pub fn export(d: &Design, c: ExportCmd, pending: &Pending) -> Result<(Vec<Op>, Write)> {
    let (brick, add, w) = match c {
        ExportCmd::Set { brick, w } => (brick, true, w),
        ExportCmd::Rm { brick, w } => (brick, false, w),
    };
    let (file, name) = resolve_pending(d, &brick_path(&brick), pending, false)?;
    let op = match add {
        true => Op::AddExport { file, brick: name },
        false => Op::RemoveExport { file, brick: name },
    };
    Ok((vec![op], w))
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
        scope: l.scope.clone(),
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
