//! Text helpers of the docs page: HTML escaping and doc-comment markdown.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};

/// `s` escaped for HTML text and attribute values.
pub fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&#39;"),
            c => o.push(c),
        }
    }
    o
}

/// Rustdoc attributes that may appear in a fence's info string.
fn rustdoc_attr(t: &str) -> bool {
    matches!(
        t,
        "rust" | "no_run" | "ignore" | "should_panic" | "compile_fail" | "test_harness"
    ) || t.starts_with("ignore-")
        || t.starts_with("edition20")
        || t.starts_with("E0")
}

/// Whether a fence info string marks Rust code, as rustdoc decides it.
fn is_rust(info: &str) -> bool {
    info.split(|c: char| c == ',' || c.is_whitespace())
        .filter(|t| !t.is_empty())
        .all(rustdoc_attr)
}

/// `code` without doctest hidden lines (`#` or `# ...`); `##` stands for a literal `#`.
fn unhide(code: &str) -> String {
    let mut o = String::new();
    for line in code.split_inclusive('\n') {
        let t = line.trim_start();
        if t.trim_end() == "#" || t.starts_with("# ") {
            continue;
        }
        match t.strip_prefix("##") {
            Some(rest) => {
                o.push_str(&line[..line.len() - t.len()]);
                o.push('#');
                o.push_str(rest);
            }
            None => o.push_str(line),
        }
    }
    o
}

/// A doc comment rendered as HTML. Raw HTML in the comment is shown as text, so a doc comment
/// cannot inject markup into the page. Fences follow rustdoc: no language or only rustdoc
/// attributes means Rust (highlighted, hidden lines dropped), any other language stays plain.
pub fn markdown(doc: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut events: Vec<Event> = Vec::new();
    let mut block: Option<(bool, String)> = None;
    for e in Parser::new_ext(doc, opts) {
        match e {
            Event::Start(Tag::CodeBlock(kind)) => {
                let rust = match &kind {
                    CodeBlockKind::Fenced(info) => is_rust(info),
                    CodeBlockKind::Indented => true,
                };
                block = Some((rust, String::new()));
                if !rust {
                    events.push(Event::Start(Tag::CodeBlock(kind)));
                }
            }
            Event::End(TagEnd::CodeBlock) => match block.take() {
                Some((true, code)) => {
                    let h = super::highlight::rust(&unhide(&code));
                    events.push(Event::Html(
                        format!("<pre class=\"rust\"><code>{h}</code></pre>\n").into(),
                    ));
                }
                _ => events.push(e),
            },
            Event::Text(t) if matches!(block, Some((true, _))) => {
                if let Some((_, code)) = &mut block {
                    code.push_str(&t);
                }
            }
            Event::Html(h) | Event::InlineHtml(h) => events.push(Event::Text(h)),
            e => events.push(e),
        }
    }
    let mut out = String::new();
    html::push_html(&mut out, events.into_iter());
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn markdown_escapes_raw_html() {
        let h = super::markdown("Uses `x`.\n\n<script>alert(1)</script>");
        assert!(h.contains("<code>x</code>"), "{h}");
        assert!(!h.contains("<script>"), "{h}");
        assert_eq!(super::esc("<a & \"b\">"), "&lt;a &amp; &quot;b&quot;&gt;");
    }

    #[test]
    fn rust_fences_hide_lines_and_highlight() {
        let doc = "```\n# use x::Y;\n## not hidden\nlet a = 1;\n```\n\n```text\n# keep\n```\n\n```ignore,edition2021\nfn f() {}\n```";
        let h = super::markdown(doc);
        assert!(!h.contains("use x::Y"), "{h}");
        assert!(h.contains("# not hidden"), "{h}");
        assert!(h.contains("h-k\">let</span>"), "{h}");
        assert!(
            h.contains("<pre><code class=\"language-text\"># keep"),
            "{h}"
        );
        assert!(h.contains("h-k\">fn</span>"), "{h}");
    }
}
