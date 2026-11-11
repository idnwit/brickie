//! The brick index by tag (the original's "bricks by family"): a tag header selects the row.

use crate::model::{BrickView, fmt_score};
use crate::state::*;
use sycamore::prelude::*;

/// The brick index by tag (the original's "bricks by family"): a tag header selects the row.
pub fn brick_index(cx: &Cx) -> View {
    let g = &cx.m.g;
    let initial = cx.scores.get_clone_untracked();
    let mut out: Vec<View> = vec![
        view! { h3(class="idx") { "Bricks by tag " span(class="muted") { "(click a tag: select the row, Shift: add)" } } },
    ];
    for tag in &g.rows {
        let (c1, c2, t) = (cx.clone(), cx.clone(), tag.clone());
        let t2 = tag.clone();
        let cls = move || {
            let on = c1
                .eval
                .with(|ev| matches!(&ev.sel, Sel::Group { tags, .. } if tags.contains(&t2)));
            if on { "tag-h on" } else { "tag-h" }
        };
        let href = Sel::None
            .select_tag(tag, false)
            .apply(&cx.params.get_clone(), g)
            .url();
        let label = tag.clone();
        out.push(view! {
            h3(class=cls, "data-g"=t.clone()) {
                a(href=href, on:click={
                    let t = t.clone();
                    move |e: Click| {
                        e.prevent_default();
                        let sel = c2.eval.with(|ev| ev.sel.clone());
                        c2.select(sel.select_tag(&t, e.shift_key()));
                    }
                }) { (label) }
            }
        });
        // Worst first inside the tag, by the score at load (stable: config order on ties).
        let mut members: Vec<(usize, &BrickView)> = g
            .bricks
            .iter()
            .enumerate()
            .filter(|(_, b)| b.tag == *tag)
            .collect();
        members.sort_by(|a, b| initial[b.0].score.total_cmp(&initial[a.0].score));
        for (b, brick) in members {
            let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
            let on = move || {
                let inside = c1.eval.with(|ev| {
                    ev.sel.set(&c1.m.g).contains(&b)
                        || ev
                            .sel
                            .scope(&c1.m.g)
                            .is_some_and(|s| s.contains(&b) && matches!(ev.sel, Sel::Edge(_)))
                });
                if inside { "item sel" } else { "item" }
            };
            let tip = "open errors (E) and warnings (W) of any layer, then the score = errors + 0.25 x warnings + complexity excess; a cycle counts once for each brick in it";
            let score = move || {
                c2.scores.with(|s| {
                    let s = &s[b];
                    if s.clean() && s.excess == 0.0 {
                        "\u{00B7} \u{2713}".to_string()
                    } else {
                        format!(
                            "\u{00B7} {}E {}W \u{00B7} {}",
                            s.errors,
                            s.warns,
                            fmt_score(s.score)
                        )
                    }
                })
            };
            let sc_cls = {
                let c = cx.clone();
                move || {
                    c.scores.with(|s| match (s[b].errors, s[b].warns) {
                        (0, 0) => "sc ok",
                        (0, _) => "sc warn",
                        _ => "sc err",
                    })
                }
            };
            let href = Sel::Brick(b).apply(&cx.params.get_clone(), g).url();
            let (name, name2) = (brick.name.clone(), brick.name.clone());
            let members = brick.members.join(", ");
            out.push(view! {
                a(class=on, href=href, "data-b"=name2, on:click=move |e: Click| {
                    e.prevent_default();
                    let sel = c3.eval.with(|ev| ev.sel.clone());
                    let next = if e.shift_key() && matches!(sel, Sel::Brick(_) | Sel::Group { .. }) { sel.add_brick(b) } else { Sel::Brick(b) };
                    c3.select(next);
                }) {
                    (name) " " span(class=sc_cls, title=tip) { (score) }
                    br()
                    span(class="muted small") { (members) }
                }
            });
        }
    }
    View::from(out)
}
