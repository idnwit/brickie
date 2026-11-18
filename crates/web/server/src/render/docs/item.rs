//! The item sections of a docs page: items grouped by kind, associated items under their type,
//! each with signature, full doc, source link and its uses.

use super::text::{esc, markdown};
use brickie_web_api::{DocsItem, DocsUse, code_href, docs_href};
use std::fmt::Write;

/// Sections in display order: (kind, heading). Associated items whose type is not listed fall
/// into the last one.
pub const SECTIONS: &[(&str, &str)] = &[
    ("struct", "Structs"),
    ("enum", "Enums"),
    ("union", "Unions"),
    ("trait", "Traits"),
    ("type", "Type aliases"),
    ("fn", "Functions"),
    ("const", "Constants"),
    ("static", "Statics"),
    ("assoc", "Associated items"),
];

const TYPES: &[&str] = &["struct", "enum", "union", "trait", "type"];

/// The type an associated item belongs to (`S` of `S::new`).
fn owner(it: &DocsItem) -> Option<&str> {
    it.name.split_once("::").map(|(t, _)| t)
}

/// The section of `it`, given the type names declared on the page.
fn section<'a>(it: &'a DocsItem, types: &[&str]) -> &'a str {
    match owner(it) {
        Some(t) if types.contains(&t) => "",
        Some(_) => "assoc",
        None => it.kind.as_str(),
    }
}

pub fn anchor(it: &DocsItem) -> String {
    format!("{}.{}", it.kind, it.name.replace("::", "."))
}

/// Items grouped by section, sorted by name; associated items of a listed type are left out
/// (they render inside their type).
pub fn grouped(items: &[DocsItem]) -> Vec<(&'static str, Vec<&DocsItem>)> {
    let types: Vec<&str> = items
        .iter()
        .filter(|i| TYPES.contains(&i.kind.as_str()))
        .map(|i| i.name.as_str())
        .collect();
    SECTIONS
        .iter()
        .filter_map(|&(kind, head)| {
            let mut v: Vec<&DocsItem> = items
                .iter()
                .filter(|i| section(i, &types) == kind)
                .collect();
            v.sort_by(|a, b| a.name.cmp(&b.name));
            (!v.is_empty()).then_some((head, v))
        })
        .collect()
}

fn uses_html(o: &mut String, uses: &[DocsUse]) {
    if uses.is_empty() {
        o.push_str("<p class=\"muted\">No use outside the brick and no test.</p>");
        return;
    }
    o.push_str("<details class=\"uses\"><summary>Used by ");
    let names: Vec<String> = uses
        .iter()
        .map(|u| format!("{} ({})", esc(short(&u.brick)), u.count))
        .collect();
    o.push_str(&names.join(", "));
    o.push_str("</summary>");
    for u in uses {
        let link = if u.brick.starts_with("root") && !u.brick.contains(' ') {
            format!(
                "<a href=\"{}\">{}</a>",
                esc(&docs_href(&u.brick)),
                esc(short(&u.brick))
            )
        } else {
            esc(&u.brick)
        };
        let _ = write!(
            o,
            "<div class=\"use\"><b>{link}</b> <span class=\"muted\">{} site(s)</span>",
            u.count
        );
        for e in &u.examples {
            let s = &e.site;
            let href = code_href(&s.file, s.line, Some(s.col));
            let _ = write!(
                o,
                "<div class=\"ex\"><a class=\"loc\" href=\"{}\">{}:{}</a><pre><code>{}</code></pre></div>",
                esc(&href),
                esc(&s.file),
                s.line,
                esc(&e.code)
            );
        }
        o.push_str("</div>");
    }
    o.push_str("</details>");
}

fn short(brick: &str) -> &str {
    brick.strip_prefix("root/").unwrap_or(brick)
}

/// One item: heading with a source link, signature, doc, uses, then its associated items.
pub fn item_html(o: &mut String, it: &DocsItem, all: &[DocsItem], nested: bool) {
    let (tag, cls) = if nested {
        ("h4", "item assoc")
    } else {
        ("h3", "item")
    };
    let href = code_href(&it.file, it.line, None);
    let _ = write!(
        o,
        "<section class=\"{cls}\" id=\"{id}\"><{tag}><span class=\"kind\">{kind}</span> \
<a href=\"#{id}\">{name}</a><a class=\"src\" href=\"{href}\">source</a></{tag}>\
<pre class=\"sig\"><code>{sig}</code></pre>",
        id = esc(&anchor(it)),
        kind = esc(&it.kind),
        name = esc(&it.name),
        href = esc(&href),
        sig = esc(&it.signature),
    );
    if !it.doc.is_empty() {
        let _ = write!(o, "<div class=\"doc\">{}</div>", markdown(&it.doc));
    }
    uses_html(o, &it.uses);
    if TYPES.contains(&it.kind.as_str()) {
        let prefix = format!("{}::", it.name);
        let mut assoc: Vec<&DocsItem> =
            all.iter().filter(|a| a.name.starts_with(&prefix)).collect();
        assoc.sort_by(|a, b| a.name.cmp(&b.name));
        if !assoc.is_empty() {
            o.push_str("<div class=\"impl\"><h4 class=\"implh\">Associated items</h4>");
            for a in assoc {
                item_html(o, a, all, true);
            }
            o.push_str("</div>");
        }
    }
    o.push_str("</section>");
}
