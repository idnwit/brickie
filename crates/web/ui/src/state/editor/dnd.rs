//! Drag and drop like folders and lanes: what a carried brick tile or a dragged brick would do
//! on release (precedence: brick body > new-lane zone > frame edge / breadcrumb > tag lane >
//! empty canvas), the label its ghost shows, and the retag and add-inside ops it records.

use super::MoveTarget;
use crate::layout::{Rect, RowOut};
use brickie_web_api::BrickView;
use brickie_web_api::design::{BrickKind, BrickSpec, Op};

/// Label of the lane of bricks without tags.
pub const UNTAGGED: &str = "(untagged)";

/// Height (world units) of the "+ new tag lane" zone under the last lane.
const NEW_LANE_H: f64 = 40.0;
/// Room under the frame that fitting the view keeps visible in edit mode: the new-lane zone.
pub const NEW_LANE_ROOM: f64 = 8.0 + NEW_LANE_H + 12.0;
/// Space between the children and the level frame (narrow at the sides: the row captions sit
/// there).
pub const FRAME_PAD: f64 = 20.0;
pub const FRAME_PADX: f64 = 10.0;

/// What is being dragged: an existing brick of the level, a new tile from the toolbar, or a
/// brick lifted from another level (it can only go into a brick, up, or onto this level).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Carried {
    Brick(usize),
    Tile,
    Lifted,
}

/// Where the pointer is, as the hit tests found it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Under {
    /// The brick body under the pointer (faded neighbours included, the dragged brick not).
    pub brick: Option<usize>,
    /// On the "+ new tag lane" zone.
    pub new_lane: bool,
    /// The ancestor level of a breadcrumb or of the frame edge under the pointer.
    pub up: Option<String>,
    /// The tag lane under the pointer: `Some(None)` is the untagged lane.
    pub lane: Option<Option<String>>,
}

/// The tag lane at world height `y` (inside the frame): the row there, else the nearest one
/// (bricks placed by hand can sit between the rows' bands). `Some(None)` is the untagged
/// lane; rows that are not lanes of `tags` (inbound sources) are no lane.
pub fn lane_hit(rows: &[RowOut], tags: &[String], y: f64) -> Option<Option<String>> {
    let gap = |r: &RowOut| (r.y - y).max(y - r.y - r.h).max(0.0);
    let r = rows.iter().min_by(|a, b| gap(a).total_cmp(&gap(b)))?;
    if r.label == UNTAGGED {
        Some(None)
    } else {
        tags.contains(&r.label).then(|| Some(r.label.clone()))
    }
}

/// The lane band of `lane` across the frame (for its highlight).
pub fn lane_rect(rows: &[RowOut], frame: &Rect, lane: Option<&str>) -> Option<Rect> {
    let label = lane.unwrap_or(UNTAGGED);
    let r = rows.iter().find(|r| r.label == label)?;
    Some(Rect {
        x: frame.x,
        y: r.y,
        w: frame.w,
        h: r.h,
    })
}

/// The level frame around the boxes of the children (faded neighbours and `skip` left out;
/// `None` when nothing is left). A drag aims with the dragged brick left out: the frame, and
/// the new-lane zone under it, must not follow the brick it is dragged to.
pub fn frame_of(bricks: &[BrickView], pos: &[Rect], skip: Option<usize>) -> Option<Rect> {
    let inner: Vec<&Rect> = (0..bricks.len().min(pos.len()))
        .filter(|&i| !bricks[i].ghost && Some(i) != skip)
        .map(|i| &pos[i])
        .collect();
    let x0 = inner.iter().map(|r| r.x).reduce(f64::min)?;
    let y0 = inner.iter().map(|r| r.y).reduce(f64::min)?;
    let x1 = inner.iter().map(|r| r.x + r.w).reduce(f64::max)?;
    let y1 = inner.iter().map(|r| r.y + r.h).reduce(f64::max)?;
    Some(Rect {
        x: x0 - FRAME_PADX,
        y: y0 - FRAME_PAD,
        w: x1 - x0 + 2.0 * FRAME_PADX,
        h: y1 - y0 + 2.0 * FRAME_PAD,
    })
}

/// The "+ new tag lane" zone of a drag (`skip`: the dragged brick), from the boxes.
pub fn new_lane_zone(
    rows: &[RowOut],
    bricks: &[BrickView],
    pos: &[Rect],
    skip: Option<usize>,
) -> Option<Rect> {
    frame_of(bricks, pos, skip).map(|f| new_lane_rect(rows, &f))
}

/// The "+ new tag lane" zone: a band under the lowest row (or the frame), as wide as the frame.
pub fn new_lane_rect(rows: &[RowOut], frame: &Rect) -> Rect {
    let bottom = rows
        .iter()
        .map(|r| r.y + r.h)
        .fold(frame.y + frame.h, f64::max);
    Rect {
        x: frame.x,
        y: bottom + 8.0,
        w: frame.w,
        h: NEW_LANE_H,
    }
}

/// What releasing `what` over `under` would do; `None`: place / reposition where dropped.
pub fn drop_target(bricks: &[BrickView], what: Carried, under: &Under) -> Option<MoveTarget> {
    let placeholder = matches!(what, Carried::Brick(b) if bricks[b].unassigned);
    if let Some(t) = under.brick {
        let b = &bricks[t];
        return Some(if b.ghost {
            MoveTarget::No("a faded neighbour is outside this level".into())
        } else if b.unassigned {
            MoveTarget::No("unassigned modules are not a brick".into())
        } else if Carried::Brick(t) == what {
            MoveTarget::No("cannot go into itself".into())
        } else {
            MoveTarget::Into(t)
        });
    }
    if placeholder {
        // The unassigned placeholder only goes into a brick (its members move there).
        return None;
    }
    if under.new_lane && what != Carried::Lifted {
        return Some(MoveTarget::NewLane);
    }
    if let (Carried::Brick(_) | Carried::Lifted, Some(up)) = (what, &under.up) {
        return Some(MoveTarget::Up(up.clone()));
    }
    match (what, under.lane.clone()) {
        (Carried::Lifted, _) => None,
        (Carried::Brick(b), Some(l)) if bricks[b].tags.first() == l.as_ref() => None,
        (_, Some(l)) => Some(MoveTarget::Lane(l)),
        _ => None,
    }
}

/// The result label on the drag ghost.
pub fn drop_label(bricks: &[BrickView], what: Carried, t: Option<&MoveTarget>) -> String {
    match t {
        Some(MoveTarget::Into(i)) if matches!(what, Carried::Brick(b) if bricks[b].unassigned) => {
            format!("assign to {}", bricks[*i].name)
        }
        Some(MoveTarget::Into(i)) => {
            let b = &bricks[*i];
            if b.file.is_some() {
                format!("into {} ({} inside)", b.name, b.children.len())
            } else {
                format!("into {} (becomes a group)", b.name)
            }
        }
        Some(MoveTarget::Lane(Some(t))) => format!("tag: {t}"),
        Some(MoveTarget::Lane(None)) if what == Carried::Tile => "untagged".into(),
        Some(MoveTarget::Lane(None)) => "remove first tag".into(),
        Some(MoveTarget::NewLane) => "new tag lane\u{2026}".into(),
        Some(MoveTarget::Up(l)) => format!("up to {l}"),
        Some(MoveTarget::No(why)) => format!("\u{2715} {why}"),
        None if what == Carried::Tile => "place here".into(),
        None => "move here".into(),
    }
}

/// The `set_brick {members}` that adds the unassigned placeholder's members to brick `into`
/// (of level file `file`).
pub fn assign_op(file: &str, into: &BrickView, placeholder: &BrickView) -> Op {
    let mut members = into.members.clone();
    members.extend(
        placeholder
            .members
            .iter()
            .filter(|m| !into.members.contains(m))
            .cloned(),
    );
    Op::SetBrick {
        file: file.to_string(),
        name: into.name.clone(),
        desc: None,
        tags: None,
        members: Some(members),
        path: None,
    }
}

/// `tags` with its first tag replaced by `lane` (kept once, others kept); `None` drops it.
pub fn retag(tags: &[String], lane: Option<&str>) -> Vec<String> {
    let rest = tags.iter().skip(1).filter(|t| Some(t.as_str()) != lane);
    match lane {
        Some(l) => std::iter::once(l.to_string())
            .chain(rest.cloned())
            .collect(),
        None => rest.cloned().collect(),
    }
}

/// The `set_brick {tags}` that moves brick `b` (of `file`) to lane `lane`; `None`: no change.
pub fn retag_op(file: &str, b: &BrickView, lane: Option<&str>) -> Option<Op> {
    let tags = retag(&b.tags, lane);
    (tags != b.tags).then(|| Op::SetBrick {
        file: file.to_string(),
        name: b.name.clone(),
        desc: None,
        tags: Some(tags),
        members: None,
        path: None,
    })
}

/// The config file of the level inside brick `name` of level file `file`: its own file when
/// it is a composite, else the one `make_composite` (path = name) creates.
pub fn child_file(file: &str, b: &BrickView) -> String {
    if let Some(f) = &b.file {
        return f.clone();
    }
    match file.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/{}/brickie.toml", b.name),
        None => format!("{}/brickie.toml", b.name),
    }
}

/// Ops that add a new brick `spec` inside brick `parent` of level file `file`: make it a
/// composite first when it is a leaf.
pub fn add_inside_ops(file: &str, parent: &BrickView, spec: BrickSpec) -> Vec<Op> {
    let mut ops = Vec::new();
    if parent.file.is_none() {
        ops.push(Op::MakeComposite {
            file: file.to_string(),
            name: parent.name.clone(),
            path: parent.name.clone(),
        });
    }
    ops.push(Op::AddBrick {
        file: child_file(file, parent),
        brick: spec,
    });
    ops
}

/// The new brick of a tile drop: name, kind, members (`a, b`) and tags.
pub fn tile_spec(kind: BrickKind, name: &str, members: &str, tags: Vec<String>) -> BrickSpec {
    BrickSpec {
        name: name.trim().to_string(),
        kind: Some(kind),
        members: super::split_list(members),
        tags,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
