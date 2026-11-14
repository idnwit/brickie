//! The Files tab: the includes of the level's file (add an existing file, or create a new
//! preset with `create_file` + `add_include`; `x` removes) and which files are shared by
//! several levels.

use super::{op, ops_then};
use crate::state::*;
use brickie_web_api::design::Op;
use sycamore::prelude::*;

/// `path` (workspace-relative) as written from `file`'s directory.
pub fn rel_from(file: &str, path: &str) -> String {
    let dir: Vec<&str> = file.split('/').collect();
    let dir = &dir[..dir.len().saturating_sub(1)];
    let p: Vec<&str> = path.split('/').collect();
    let common = dir.iter().zip(&p).take_while(|(a, b)| a == b).count();
    let mut out: Vec<&str> = vec![".."; dir.len() - common];
    out.extend(&p[common..]);
    out.join("/")
}

fn include_row(cx: &Cx, file: &str, inc: String, target: String) -> View {
    let n = cx.m.design.as_deref().map_or(0, |ch| sharers(ch, &target));
    let warn =
        (n > 1).then(|| view! { span(class="warn-text") { (format!(" shared by {n} levels")) } });
    let (c, f, p) = (cx.clone(), file.to_string(), inc.clone());
    let rm = move |_: Click| {
        op(
            &c,
            Op::RemoveInclude {
                file: f.clone(),
                path: p.clone(),
            },
        )
    };
    view! { li { span(class="mono") { (inc) } (warn) button(class="px", title="remove include", on:click=rm) { "\u{00D7}" } } }
}

fn add_form(cx: &Cx, file: String, others: Vec<String>) -> View {
    let pick = create_signal(String::new());
    let fresh = create_signal(String::new());
    let opts = View::from(
        others
            .iter()
            .map(|o| {
                let (v, t) = (o.clone(), o.clone());
                view! { option(value=v) { (t) } }
            })
            .collect::<Vec<_>>(),
    );
    let (c, f) = (cx.clone(), file.clone());
    let add = move |_: Click| {
        let p = pick.get_clone();
        if !p.is_empty() {
            op(
                &c,
                Op::AddInclude {
                    file: f.clone(),
                    path: rel_from(&f, &p),
                },
            );
        }
    };
    let (c, f) = (cx.clone(), file);
    let create = move |_: Click| {
        let p = fresh.get_clone().trim().to_string();
        if p.is_empty() {
            return;
        }
        let path = rel_from(&f, &p);
        let f2 = f.clone();
        ops_then(&c, vec![Op::CreateFile { file: p }], move |now, _| {
            op(
                now,
                Op::AddInclude {
                    file: f2.clone(),
                    path: path.clone(),
                },
            )
        });
    };
    view! {
        div(class="row") { select(bind:value=pick) { option(value="") { "existing file\u{2026}" } (opts) } button(on:click=add) { "+ include" } }
        div(class="row") { input(r#type="text", placeholder="new preset: presets/strict.toml", bind:value=fresh) button(on:click=create) { "New preset" } }
    }
}

pub fn files(cx: &Cx) -> View {
    let (Some(ch), Some(file)) = (cx.m.design.clone(), cx.level_file()) else {
        return View::default();
    };
    let sheet = ch.sheets.get(&file).cloned().unwrap_or_default();
    let rows: Vec<View> = sheet
        .include
        .iter()
        .cloned()
        .zip(sheet.include_files.iter().cloned())
        .map(|(i, t)| include_row(cx, &file, i, t))
        .collect();
    let empty = rows
        .is_empty()
        .then(|| view! { li(class="muted") { "no includes" } });
    let others: Vec<String> = ch
        .sheets
        .keys()
        .filter(|k| **k != file && !sheet.include_files.contains(k))
        .cloned()
        .collect();
    let n = sharers(&ch, &file);
    let shared_text = format!("{file} is read by {n} levels.");
    let shared = (n > 1).then(|| view! { p(class="warn-text") { (shared_text) } });
    let form = add_form(cx, file.clone(), others);
    let rows = View::from(rows);
    view! {
        h3 { "Files" }
        p(class="mono") { (file) }
        (shared)
        label { "include" }
        ul(class="incs") { (rows) (empty) }
        (form)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn includes_are_written_relative_to_the_file() {
        use super::rel_from;
        assert_eq!(
            rel_from("web/brickie.toml", "presets/a.toml"),
            "../presets/a.toml"
        );
        assert_eq!(rel_from("brickie.toml", "presets/a.toml"), "presets/a.toml");
        assert_eq!(rel_from("web/brickie.toml", "web/x.toml"), "x.toml");
    }
}
