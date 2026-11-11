//! The functions table rows from the cache.

use crate::check::Loaded;
use brickie_ui::*;

pub fn functions(l: &Loaded) -> Vec<FnRow> {
    let mut out = Vec::new();
    for (path, f) in &l.cache.files {
        for func in &f.functions {
            let brick = func
                .module
                .as_ref()
                .and_then(|m| l.cache.bricks.module_brick.get(m))
                .cloned()
                .unwrap_or_default();
            out.push(FnRow {
                qname: func.qname.clone(),
                file: path.clone(),
                line: func.line,
                brick,
                scope: func.scope.clone(),
                cognitive: func.cognitive,
                cyclomatic: func.cyclomatic,
                nesting: func.nesting,
                paths: func.paths,
                params: func.params,
                lines: func.lines,
            });
        }
    }
    out
}
