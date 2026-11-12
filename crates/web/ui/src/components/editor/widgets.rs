//! Small form widgets of the design panel. Every field applies itself (one op) on blur or
//! Enter when its value changed; there is no Save button.

use crate::state::Click;
use std::rc::Rc;
use sycamore::prelude::*;

/// A text field committing on blur / Enter (`textarea` when `multi`).
pub fn text_field(
    id: &'static str,
    label: &'static str,
    init: String,
    multi: bool,
    commit: impl Fn(String) + 'static,
) -> View {
    let v = create_signal(init.clone());
    let commit = Rc::new(commit);
    let last = create_signal(init);
    let fire = {
        let commit = commit.clone();
        move || {
            // A blur can come from the field being removed by a re-render.
            if !v.is_alive() {
                return;
            }
            let now = v.get_clone();
            if now != last.get_clone() {
                last.set(now.clone());
                commit(now);
            }
        }
    };
    let f2 = fire.clone();
    let key = move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            f2();
        }
    };
    let blur = move |_: web_sys::FocusEvent| fire();
    let input = if multi {
        view! { textarea(id=id, class="desc", bind:value=v, on:blur=blur, on:keydown=key) }
    } else {
        view! { input(id=id, r#type="text", bind:value=v, on:blur=blur, on:keydown=key) }
    };
    view! { label { (label) } (input) }
}

/// A list of pills with `x` to remove and an input to add (Enter): `change(new list)`.
pub fn chips_field(
    id: &'static str,
    label: &'static str,
    items: Vec<String>,
    hint: &'static str,
    change: impl Fn(Vec<String>) + 'static,
) -> View {
    let change = Rc::new(change);
    let pills: Vec<View> = items
        .iter()
        .map(|t| {
            let (t, items, change) = (t.clone(), items.clone(), change.clone());
            let label = t.clone();
            let rm = move |_: Click| change(items.iter().filter(|x| **x != t).cloned().collect());
            view! { span(class="pill") { (label) button(class="px", title="remove", on:click=rm) { "\u{00D7}" } } }
        })
        .collect();
    let add = create_signal(String::new());
    let key = move |e: web_sys::KeyboardEvent| {
        let t = add.get_clone().trim().to_string();
        if e.key() == "Enter" && !t.is_empty() && !items.contains(&t) {
            let mut next = items.clone();
            next.push(t);
            add.set(String::new());
            change(next);
        }
    };
    let pills = View::from(pills);
    view! {
        label { (label) }
        div(class="pills") { (pills) input(id=id, class="padd", r#type="text", placeholder=hint, bind:value=add, on:keydown=key) }
    }
}

/// A segmented choice; `pick(option)` when another one is clicked.
pub fn seg(options: &[&'static str], current: &str, pick: impl Fn(&'static str) + 'static) -> View {
    let pick = Rc::new(pick);
    let items: Vec<View> = options
        .iter()
        .map(|&o| {
            let on = o == current;
            let pick = pick.clone();
            let cls = if on { "segb on" } else { "segb" };
            view! { button(class=cls, on:click=move |_: Click| if !on { pick(o) }) { (o) } }
        })
        .collect();
    let items = View::from(items);
    view! { span(class="seg") { (items) } }
}

/// `Some(n)` of a number field (empty: unset).
pub fn num(s: &str) -> Option<u64> {
    s.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    #[test]
    fn numbers_parse_or_unset() {
        assert_eq!(super::num(" 12 "), Some(12));
        assert_eq!(super::num(""), None);
    }
}
