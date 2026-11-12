//! Page keys (registered once, acting on the level on screen): presets `1`-`3`, `/` search,
//! `j`/`k` through the findings, `Escape` drops the editor's armed tool or open form, closes
//! the popup, then the finding, then the selection and finally zooms out; `Backspace` zooms
//! out.

use super::panels::current_list;
use crate::platform;
use crate::state::*;
use brickie_web_api::Params;

/// Zoom out to the parent level (no-op at the root).
pub fn zoom_out(c: &Cx) -> bool {
    let p = c.params.get_clone();
    match p.parent_level() {
        Some(up) => {
            let next = p.at_level(up);
            c.go(next);
            true
        }
        None => false,
    }
}

/// `Escape`, in order: cancel the editor's drag or armed tool/form (true: consumed), else close
/// the popup, else the focused finding, else the selection, else zoom out.
fn on_escape(c: &Cx, p: &Params) -> bool {
    if crate::components::editor::cancel_drag(c) || c.ed.cancel() {
        return true;
    }
    let selected = !p.brick.is_empty() || !p.group.is_empty() || !p.edge.is_empty();
    if c.modal.get_clone().is_some() {
        c.modal.set(None);
    } else if !p.item.is_empty() {
        c.go(p.with(|p| p.item.clear()));
    } else if selected {
        c.select(Sel::None);
    } else {
        zoom_out(c);
    }
    false
}

/// The finding to move to in a list of `len` (not empty) from position `at` (`None`: the
/// current one is not listed, so start at the first): next or previous, stopping at the ends.
fn step(at: Option<usize>, len: usize, forward: bool) -> usize {
    if forward {
        at.map_or(0, |i| (i + 1).min(len - 1))
    } else {
        at.map_or(0, |i| i.saturating_sub(1))
    }
}

pub fn keys(cur: &Cur) {
    let cur = cur.clone();
    platform::on_key(move |key, in_input| {
        if in_input {
            return false;
        }
        // A key the editor handles is not a page key.
        let Some(c) = current(&cur).filter(|c| crate::components::editor::key(c, &key).is_none())
        else {
            return false;
        };
        let p = c.params.get_clone();
        match key.as_str() {
            "1" | "2" | "3" => {
                let name = ["arch", "viol", "all"][key.as_bytes()[0] as usize - b'1' as usize];
                c.go(with_preset(&p, name));
            }
            "Escape" => return on_escape(&c, &p),
            "Backspace" => return zoom_out(&c),
            "/" => {
                platform::focus("q");
                return true;
            }
            "j" | "k" => {
                let list = current_list(&c);
                if list.is_empty() {
                    return false;
                }
                let at = list.iter().position(|&i| c.m.diags[i].id == p.item);
                let next = step(at, list.len(), key == "j");
                let id = c.m.diags[list[next]].id.clone();
                c.go(p.with(|p| p.item = id));
            }
            _ => {}
        }
        false
    });
}

#[cfg(test)]
mod tests {
    use super::step;

    #[test]
    fn step_clamps_at_the_ends_and_starts_at_the_first() {
        // Not in the list: both directions start at the first.
        assert_eq!(step(None, 3, true), 0);
        assert_eq!(step(None, 3, false), 0);
        assert_eq!(step(Some(0), 3, true), 1);
        assert_eq!(step(Some(1), 3, false), 0);
        // Clamped: no wrap-around.
        assert_eq!(step(Some(2), 3, true), 2);
        assert_eq!(step(Some(0), 3, false), 0);
        assert_eq!(step(Some(0), 1, true), 0);
    }
}
