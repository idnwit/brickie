//! Text helpers of the docs page: HTML escaping and doc-comment markdown.

use pulldown_cmark::{Event, Options, Parser, html};

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

/// A doc comment rendered as HTML. Raw HTML in the comment is shown as text, so a doc comment
/// cannot inject markup into the page.
pub fn markdown(doc: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let events = Parser::new_ext(doc, opts).map(|e| match e {
        Event::Html(h) | Event::InlineHtml(h) => Event::Text(h),
        e => e,
    });
    let mut out = String::new();
    html::push_html(&mut out, events);
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
}
