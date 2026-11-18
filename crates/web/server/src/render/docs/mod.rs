//! The docs page of a brick (`GET /docs/<brick path>`), docs.rs style: a sidebar with the brick
//! tree and the page's sections, and the brick's children and declared items. Plain HTML, no
//! JavaScript.

mod item;
mod text;

use brickie_web_api::{DocsNode, DocsPage, PAGE_PATH, code_brick_href, docs_href};
use item::{grouped, item_html};
use std::fmt::Write;
use text::{esc, markdown};

const CSS: &str = include_str!("docs.css");

fn short(path: &str) -> &str {
    path.strip_prefix("root/").unwrap_or(path)
}

/// The sidebar: the whole tree (the current brick and its ancestors marked), then the page's
/// sections and items.
fn sidebar(o: &mut String, p: &DocsPage, sections: &[(&str, Vec<&brickie_web_api::DocsItem>)]) {
    let _ = write!(
        o,
        "<nav class=\"side\"><a class=\"home\" href=\"{}\">brickie docs</a>\
<a class=\"muted\" href=\"{}\">open the viewer</a><h4>Bricks</h4><ul class=\"tree\">",
        esc(&docs_href("root")),
        PAGE_PATH
    );
    for n in &p.tree {
        let cls = if n.path == p.brick {
            "cur"
        } else if p.brick.starts_with(&format!("{}/", n.path)) {
            "anc"
        } else {
            ""
        };
        let _ = write!(
            o,
            "<li style=\"padding-left:{}px\"><a class=\"{cls}\" href=\"{}\">{}</a></li>",
            (n.depth - 1) * 12,
            esc(&docs_href(&n.path)),
            esc(&n.name)
        );
    }
    o.push_str("</ul>");
    if !p.children.is_empty() {
        o.push_str("<h4><a href=\"#children\">Bricks</a></h4>");
    }
    for (head, items) in sections {
        let id = head.to_lowercase().replace(' ', "-");
        let _ = write!(o, "<h4><a href=\"#{id}\">{head}</a></h4><ul>");
        for it in items {
            let _ = write!(
                o,
                "<li><a href=\"#{}\">{}</a></li>",
                esc(&item::anchor(it)),
                esc(&it.name)
            );
        }
        o.push_str("</ul>");
    }
    o.push_str("</nav>");
}

fn children(o: &mut String, kids: &[DocsNode]) {
    if kids.is_empty() {
        return;
    }
    o.push_str("<h2 id=\"children\">Bricks</h2><table class=\"kids\">");
    for k in kids {
        let mark = if k.exported {
            ""
        } else {
            " <span class=\"muted\">(internal)</span>"
        };
        let _ = write!(
            o,
            "<tr><td><a href=\"{}\">{}</a>{mark}</td><td>{}</td></tr>",
            esc(&docs_href(&k.path)),
            esc(&k.name),
            esc(k.desc.as_deref().unwrap_or(""))
        );
    }
    o.push_str("</table>");
}

/// The whole docs page.
pub fn docs_html(p: &DocsPage) -> String {
    let sections = grouped(&p.items);
    let mut o = String::new();
    let title = if p.brick == "root" {
        "bricks"
    } else {
        short(&p.brick)
    };
    let _ = write!(
        o,
        "{head}<title>{t} - brickie docs</title><style>{CSS}</style></head><body>",
        head = super::SHELL_HEAD,
        t = esc(title)
    );
    sidebar(&mut o, p, &sections);
    o.push_str("<main>");
    let _ = write!(o, "<h1>Brick <span class=\"path\">{}</span>", esc(&p.brick));
    if p.brick != "root" {
        let _ = write!(
            o,
            "<a class=\"src\" href=\"{}\">code</a>",
            esc(&code_brick_href(&p.brick))
        );
    }
    o.push_str("</h1>");
    if let Some(d) = &p.desc {
        let _ = write!(o, "<div class=\"doc\">{}</div>", markdown(d));
    }
    children(&mut o, &p.children);
    if p.brick != "root" && p.items.is_empty() {
        o.push_str("<p class=\"muted\">This brick declares no items to other bricks.</p>");
    }
    for (head, items) in &sections {
        let id = head.to_lowercase().replace(' ', "-");
        let _ = write!(o, "<h2 id=\"{id}\">{head}</h2>");
        for it in items {
            item_html(&mut o, it, &p.items, false);
        }
    }
    o.push_str("</main></body></html>");
    o
}
