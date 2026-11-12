//! The source popup page: numbered lines, the target highlighted, the complexity tree of the
//! function around it (computed on demand).

use super::SHELL_HEAD;
use crate::data::split_bytes;
use brickie_ui::{SourceLine, SourcePage};

/// A click on a tree bar marks the lines it spans (`pickBar` of the original).
const BAR_JS: &str = "<script>document.addEventListener('click',function(e){var b=e.target.closest('.vb');if(!b)return;\
var a=+b.dataset.a,z=+b.dataset.b;document.querySelectorAll('.cl.sel').forEach(function(x){x.classList.remove('sel')});\
for(var i=a;i<=z;i++){var r=document.getElementById('L'+i);if(r)r.classList.add('sel')}\
var f=document.getElementById('L'+a);if(f)f.scrollIntoView({block:'center'})});</script>";

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The whole page for `text` (the file `file`) with `line` (and optionally the span
/// `col..end_col`, 1-based bytes) marked.
pub fn source_html(
    file: &str,
    text: &str,
    line: u32,
    col: Option<u32>,
    end_col: Option<u32>,
) -> String {
    let lines: Vec<SourceLine> = text
        .lines()
        .enumerate()
        .map(|(i, t)| {
            let n = i as u32 + 1;
            let on = n == line;
            match (on, col) {
                (true, Some(c)) => {
                    let len = end_col.map_or(1, |e| e.saturating_sub(c).max(1));
                    let (before, hit, after) =
                        split_bytes(t, (c as usize).saturating_sub(1), len as usize);
                    SourceLine {
                        n,
                        before,
                        hit,
                        after,
                        line_hl: true,
                    }
                }
                _ => SourceLine {
                    n,
                    before: String::new(),
                    hit: String::new(),
                    after: t.to_string(),
                    line_hl: on,
                },
            }
        })
        .collect();
    let tree = (line > 0)
        .then(|| brickie_measure::fn_tree(text, line))
        .flatten();
    let body = brickie_ui::render_source(&SourcePage {
        file: file.to_string(),
        lines,
        tree,
    });
    // The anchor works when the link has `#L<line>`; this scrolls when it has not.
    let scroll = if line > 0 {
        format!(
            "<script>var e=document.getElementById('L{line}');if(e)e.scrollIntoView({{block:'center'}});</script>"
        )
    } else {
        String::new()
    };
    format!(
        "{SHELL_HEAD}<title>{}</title><style>{}</style></head><body class=\"srcpage\">{body}{scroll}{BAR_JS}</body></html>",
        html_escape(file),
        brickie_ui::CSS,
    )
}
