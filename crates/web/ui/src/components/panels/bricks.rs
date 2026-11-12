//! The brick index by tag (the original's "bricks by family"): a tag header selects the row.

use crate::state::*;
use brickie_web_api::{BrickScore, BrickView, GraphView, fmt_score};
use sycamore::prelude::*;

const TIP: &str = "open errors (E) and warnings (W) of any layer, then the score = errors + 0.25 x warnings + complexity excess; a cycle counts once for each brick in it";

/// The brick index by tag (the original's "bricks by family"): a tag header selects the row.
pub fn brick_index(cx: &Cx) -> View {
    let g = &cx.m.g;
    let initial = cx.scores.get_clone_untracked();
    let mut out: Vec<View> = vec![
        view! { h3(class="idx") { "Bricks by tag " span(class="muted") { "(click a tag: select the row, Shift: add)" } } },
    ];
    for tag in &g.rows {
        out.push(tag_header(cx, tag));
        for (b, brick) in members_sorted(g, tag, &initial) {
            out.push(brick_row(cx, b, brick));
        }
    }
    View::from(out)
}

/// The header of one tag: highlighted while the tag is in the group selection.
fn tag_header(cx: &Cx, tag: &str) -> View {
    let (c1, c2, t) = (cx.clone(), cx.clone(), tag.to_string());
    let t2 = t.clone();
    let cls = move || {
        let on = c1
            .eval
            .with(|ev| matches!(&ev.sel, Sel::Group { tags, .. } if tags.contains(&t2)));
        if on { "tag-h on" } else { "tag-h" }
    };
    let href = Sel::None
        .select_tag(tag, false)
        .apply(&cx.params.get_clone(), &cx.m.g)
        .url();
    let label = tag.to_string();
    view! {
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
    }
}

/// One brick of the index: name, live score and members; a click selects it.
fn brick_row(cx: &Cx, b: usize, brick: &BrickView) -> View {
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
    let score = move || c2.scores.with(|s| score_label(&s[b]));
    let sc_cls = {
        let c = cx.clone();
        move || c.scores.with(|s| score_class(&s[b]))
    };
    let href = Sel::Brick(b).apply(&cx.params.get_clone(), &cx.m.g).url();
    let (name, name2) = (brick.name.clone(), brick.name.clone());
    let members = brick.members.join(", ");
    view! {
        a(class=on, href=href, "data-b"=name2, on:click=move |e: Click| {
            e.prevent_default();
            let sel = c3.eval.with(|ev| ev.sel.clone());
            c3.select(click_target(sel, e.shift_key(), b));
        }) {
            (name) " " span(class=sc_cls, title=TIP) { (score) }
            br()
            span(class="muted small") { (members) }
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
        format!(
            "\u{00B7} {}E {}W \u{00B7} {}",
            s.errors,
            s.warns,
            fmt_score(s.score)
        )
    }
}

/// The class of the score badge: ok without findings, warn with warnings only, else err.
fn score_class(s: &BrickScore) -> &'static str {
    match (s.errors, s.warns) {
        (0, 0) => "sc ok",
        (0, _) => "sc warn",
        _ => "sc err",
    }
}

/// The selection after a click on brick `b`: Shift adds it to a brick or group selection.
fn click_target(sel: Sel, shift: bool, b: usize) -> Sel {
    if shift && matches!(sel, Sel::Brick(_) | Sel::Group { .. }) {
        sel.add_brick(b)
    } else {
        Sel::Brick(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sc(errors: u32, warns: u32, excess: f64, score: f64) -> BrickScore {
        BrickScore {
            errors,
            warns,
            excess,
            score,
            ..Default::default()
        }
    }

    #[test]
    fn label_is_a_check_only_when_clean_and_within_limits() {
        assert_eq!(score_label(&sc(0, 0, 0.0, 0.0)), "\u{00B7} \u{2713}");
        assert_eq!(
            score_label(&sc(0, 0, 0.5, 0.5)),
            "\u{00B7} 0E 0W \u{00B7} 0.5"
        );
        assert_eq!(
            score_label(&sc(2, 1, 0.0, 2.25)),
            "\u{00B7} 2E 1W \u{00B7} 2.25"
        );
    }

    #[test]
    fn class_follows_errors_then_warnings() {
        assert_eq!(score_class(&sc(0, 0, 0.0, 0.0)), "sc ok");
        assert_eq!(score_class(&sc(0, 3, 0.0, 0.0)), "sc warn");
        assert_eq!(score_class(&sc(1, 0, 0.0, 0.0)), "sc err");
        assert_eq!(score_class(&sc(1, 1, 0.0, 0.0)), "sc err");
    }

    #[test]
    fn members_sort_worst_first_and_stable() {
        let initial = [sc(0, 0, 0.0, 1.0), sc(0, 0, 0.0, 3.0), sc(0, 0, 0.0, 1.0)];
        let mut m = vec![(0, ()), (1, ()), (2, ())];
        sort_worst_first(&mut m, &initial);
        let order: Vec<usize> = m.iter().map(|x| x.0).collect();
        assert_eq!(order, [1, 0, 2]);
    }

    #[test]
    fn click_replaces_unless_shift_on_brick_or_group() {
        assert_eq!(click_target(Sel::Brick(1), false, 2), Sel::Brick(2));
        assert_eq!(click_target(Sel::None, true, 2), Sel::Brick(2));
        assert_eq!(click_target(Sel::Edge(0), true, 2), Sel::Brick(2));
        assert_eq!(
            click_target(Sel::Brick(1), true, 2),
            Sel::Group {
                tags: vec![],
                bricks: vec![1, 2]
            }
        );
        assert_eq!(click_target(Sel::Brick(1), true, 1), Sel::None);
    }
}
