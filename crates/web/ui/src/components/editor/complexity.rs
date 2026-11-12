//! The complexity table of the Rules tab: one row per `[[complexity]]` rule of the level (its
//! file and includes); a cell applies on blur (`set_complexity`), `+` adds a rule, `x` removes.

use super::op;
use super::widgets::num;
use crate::state::*;
use brickie_web_api::change::{ComplexitySpec, Op};
use std::rc::Rc;
use sycamore::prelude::*;

type Set = Rc<dyn Fn(&mut ComplexitySpec, String)>;

fn columns() -> Vec<(&'static str, Set)> {
    fn n(f: fn(&mut ComplexitySpec) -> &mut Option<u64>) -> Set {
        Rc::new(move |s, v| *f(s) = num(&v))
    }
    fn t(f: fn(&mut ComplexitySpec) -> &mut Option<String>) -> Set {
        Rc::new(move |s, v| *f(s) = (!v.trim().is_empty()).then(|| v.trim().to_string()))
    }
    vec![
        ("bricks", t(|s| &mut s.bricks)),
        ("cognitive", n(|s| &mut s.cognitive)),
        ("cyclomatic", n(|s| &mut s.cyclomatic)),
        ("nesting", n(|s| &mut s.nesting)),
        ("paths", n(|s| &mut s.paths)),
        ("params", n(|s| &mut s.params)),
        ("lines", n(|s| &mut s.lines)),
        ("level", t(|s| &mut s.level)),
        ("scope", t(|s| &mut s.scope)),
        ("help", t(|s| &mut s.help)),
    ]
}

fn value(s: &ComplexitySpec, col: &str) -> String {
    let n = |v: Option<u64>| v.map(|v| v.to_string()).unwrap_or_default();
    match col {
        "bricks" => s.bricks.clone().unwrap_or_default(),
        "cognitive" => n(s.cognitive),
        "cyclomatic" => n(s.cyclomatic),
        "nesting" => n(s.nesting),
        "paths" => n(s.paths),
        "params" => n(s.params),
        "lines" => n(s.lines),
        "level" => s.level.clone().unwrap_or_default(),
        "scope" => s.scope.clone().unwrap_or_default(),
        _ => s.help.clone().unwrap_or_default(),
    }
}

fn cell(cx: &Cx, r: &Row<ComplexitySpec>, col: &'static str, set: Set) -> View {
    let init = value(&r.item, col);
    let v = create_signal(init.clone());
    let (c, file, index, base) = (cx.clone(), r.file.clone(), r.index, r.item.clone());
    let blur = move |_: web_sys::FocusEvent| {
        if !v.is_alive() {
            return;
        }
        let now = v.get_clone();
        if now != init {
            let mut rule = base.clone();
            set(&mut rule, now);
            op(
                &c,
                Op::SetComplexity {
                    file: file.clone(),
                    index,
                    rule,
                },
            );
        }
    };
    view! { td { input(r#type="text", class="cell", title=col, bind:value=v, on:blur=blur) } }
}

fn row(cx: &Cx, r: Row<ComplexitySpec>) -> View {
    let cells: Vec<View> = columns()
        .into_iter()
        .map(|(col, set)| cell(cx, &r, col, set))
        .collect();
    let (c, file, index) = (cx.clone(), r.file.clone(), r.index);
    let rm = move |_: Click| {
        op(
            &c,
            Op::RemoveComplexity {
                file: file.clone(),
                index,
            },
        )
    };
    let at = format!("{}#{}", r.file, r.index);
    let cells = View::from(cells);
    view! { tr(title=at) { (cells) td { button(class="px", title="remove", on:click=rm) { "\u{00D7}" } } } }
}

pub fn complexity(cx: &Cx) -> View {
    let (Some(ch), Some(file)) = (cx.m.change.clone(), cx.level_file()) else {
        return View::default();
    };
    let rows: Vec<View> = level_complexity(&ch, &cx.level().path)
        .into_iter()
        .map(|r| row(cx, r))
        .collect();
    let head = View::from(
        columns()
            .into_iter()
            .map(|(c, _)| view! { th { (c) } })
            .collect::<Vec<_>>(),
    );
    let rows = View::from(rows);
    let c = cx.clone();
    let add = move |_: Click| {
        let rule = ComplexitySpec {
            bricks: Some("*".into()),
            cognitive: Some(25),
            ..Default::default()
        };
        op(
            &c,
            Op::AddComplexity {
                file: file.clone(),
                rule,
            },
        );
    };
    view! {
        h3 { "Complexity rules" }
        div(class="tscroll") { table(class="rules cx") { tr { (head) th {} } (rows) } }
        button(on:click=add) { "+ complexity rule" }
        p(class="muted") { "Empty cell: unset. bricks is a selector (*, a brick, tag:<t>)." }
    }
}
