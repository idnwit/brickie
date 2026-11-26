//! The brick index by tag (the original's "bricks by family"): a click selects a brick.

use crate::state::*;
use brickie_web_api::{BrickScore, BrickView, GraphView, fmt_lines_compact, fmt_score};
use sycamore::prelude::*;

const TIP: &str = "findings (E) of any layer, then the score = findings + complexity excess; a cycle counts once for each brick in it";

/// The brick index by tag (the original's "bricks by family"); a click selects a brick.
pub fn brick_index(cx: &Cx) -> View {
    let g = &cx.m.g;
    let initial = cx.scores.get_clone_untracked();
    let mut out: Vec<View> = vec![
        view! { h3(class="idx") { "Bricks by tag " span(class="muted") { "(click a brick to select it)" } } },
    ];
    for tag in &g.rows {
        let label = tag.clone();
        out.push(view! { h3(class="tag-h") { (label) } });
        for (b, brick) in members_sorted(g, tag, &initial) {
            out.push(brick_row(cx, b, brick));
        }
    }
    View::from(out)
}

/// One brick of the index: name, live score and members; a click selects it.
fn brick_row(cx: &Cx, b: usize, brick: &BrickView) -> View {
    let (c1, c2, c3) = (cx.clone(), cx.clone(), cx.clone());
    // Only a selected brick is marked; a selected link marks none.
    let on = move || {
        if c1.eval.with(|ev| ev.sel.brick() == Some(b)) {
            "item sel"
        } else {
            "item"
        }
    };
    let score = move || c2.scores.with(|s| score_label(&s[b]));
    let sc_cls = {
        let c = cx.clone();
        move || c.scores.with(|s| score_class(&s[b]))
    };
    let href = Sel::Brick(b).apply(&cx.params.get_clone(), &cx.m.g).url();
    let (name, name2) = (brick.name.clone(), brick.name.clone());
    let members = if brick.is_group {
        brick
            .group_members
            .iter()
            .map(|m| m.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        brick.members.join(", ")
    };
    let lines = fmt_lines_compact(brick.code_lines, brick.test_lines);
    let sub = if lines.is_empty() {
        members
    } else if members.is_empty() {
        format!("{lines} lines")
    } else {
        format!("{lines} \u{00B7} {members}")
    };
    view! {
        a(class=on, href=href, "data-b"=name2, on:click=move |e: Click| {
            e.prevent_default();
            let sel = c3.eval.with(|ev| ev.sel.clone());
            c3.select(sel.click(b));
        }) {
            (name) " " span(class=sc_cls, title=TIP) { (score) }
            br()
            span(class="muted small") { (sub) }
        }
    }
}

/// The bricks of `tag` with their index, worst first by the score at load.
fn members_sorted<'a>(
    g: &'a GraphView,
    tag: &str,
    initial: &[BrickScore],
) -> Vec<(usize, &'a BrickView)> {
    let mut members: Vec<(usize, &BrickView)> = g
        .bricks
        .iter()
        .enumerate()
        .filter(|(_, b)| b.tag == tag)
        .collect();
    sort_worst_first(&mut members, initial);
    members
}

/// Worst score first, stable: config order on ties.
fn sort_worst_first<T>(members: &mut [(usize, T)], initial: &[BrickScore]) {
    members.sort_by(|a, b| initial[b.0].score.total_cmp(&initial[a.0].score));
}

/// The score text after a brick name.
fn score_label(s: &BrickScore) -> String {
    if s.clean() && s.excess == 0.0 {
        "\u{00B7} \u{2713}".to_string()
    } else {
        format!("\u{00B7} {}E \u{00B7} {}", s.errors, fmt_score(s.score))
    }
}

/// The class of the score badge: ok without findings, else err.
fn score_class(s: &BrickScore) -> &'static str {
    if s.errors == 0 { "sc ok" } else { "sc err" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(errors: u32, excess: f64, score: f64) -> BrickScore {
        BrickScore {
            errors,
            excess,
            score,
            ..Default::default()
        }
    }

    #[test]
    fn label_is_a_check_only_when_clean_and_within_limits() {
        assert_eq!(score_label(&sc(0, 0.0, 0.0)), "\u{00B7} \u{2713}");
        assert_eq!(score_label(&sc(0, 0.5, 0.5)), "\u{00B7} 0E \u{00B7} 0.5");
        assert_eq!(score_label(&sc(2, 0.0, 2.0)), "\u{00B7} 2E \u{00B7} 2");
    }

    #[test]
    fn class_follows_errors() {
        assert_eq!(score_class(&sc(0, 0.0, 0.0)), "sc ok");
        assert_eq!(score_class(&sc(1, 0.0, 0.0)), "sc err");
    }

    #[test]
    fn members_sort_worst_first_and_stable() {
        let initial = [sc(0, 0.0, 1.0), sc(0, 0.0, 3.0), sc(0, 0.0, 1.0)];
        let mut m = vec![(0, ()), (1, ()), (2, ())];
        sort_worst_first(&mut m, &initial);
        let order: Vec<usize> = m.iter().map(|x| x.0).collect();
        assert_eq!(order, [1, 0, 2]);
    }
}
