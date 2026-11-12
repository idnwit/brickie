//! The functions table rows from the cache.

use brickie_check::check::Loaded;
use brickie_web_api::*;

pub fn functions(l: &Loaded) -> Vec<FnRow> {
    let mut out = Vec::new();
    for (path, f) in &l.cache.files {
        for func in &f.functions {
            let brick = func
                .module
                .as_ref()
                .and_then(|m| l.resolved.path(m))
                .map(brickie_check::config::level_name)
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
