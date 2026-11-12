//! Page keys (registered once, acting on the level on screen): presets `1`-`3`, `/` search,
//! `j`/`k` through the findings, `Escape` closes the popup, then the finding, then the
//! selection and finally zooms out; `Backspace` zooms out.

use super::panels::current_list;
use crate::platform;
use crate::state::*;

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

pub fn keys(cur: &Cur) {
    let cur = cur.clone();
    platform::on_key(move |key, in_input| {
        if in_input {
            return false;
        }
        let Some(c) = current(&cur) else {
            return false;
        };
        let p = c.params.get_clone();
        match key.as_str() {
            "1" | "2" | "3" => {
                let name = ["arch", "viol", "all"][key.as_bytes()[0] as usize - b'1' as usize];
                c.go(with_preset(&p, name));
            }
            "Escape" => {
                let selected = !p.brick.is_empty() || !p.group.is_empty() || !p.edge.is_empty();
                if c.modal.get_clone().is_some() {
                    c.modal.set(None);
                } else if !p.item.is_empty() {
                    c.go(p.with(|p| p.item.clear()));
                } else if selected {
                    c.select(Sel::None);
                } else {
                    zoom_out(&c);
                }
            }
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
                let next = if key == "j" {
                    at.map_or(0, |i| (i + 1).min(list.len() - 1))
                } else {
                    at.map_or(0, |i| i.saturating_sub(1))
                };
                let id = c.m.diags[list[next]].id.clone();
                c.go(p.with(|p| p.item = id));
            }
            _ => {}
        }
        false
    });
}
