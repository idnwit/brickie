//! Where items can be named from. A library crate exposes its root, `pub mod` children of
//! public modules and the targets of plain `pub use` items in public modules (globs
//! included). Inside a crate, an item is visible in the subtree of a module derived from its
//! own visibility, the visibility of the modules around it and `pub use` re-exports.

use crate::scip::Graph;
use crate::scip::modules::{ModuleIds, module_of};
use std::collections::{HashMap, HashSet};

pub struct Reach {
    /// Public module ids.
    modules: HashSet<String>,
    /// Symbols re-exported by a plain `pub use` in a public module.
    reexported: HashSet<usize>,
    /// (module, name) of every re-exported symbol, for associated items of re-exported types.
    reexported_names: HashSet<(String, String)>,
    /// Module id -> its declared visibility (`pub`, `pub(crate)`, `pub(super)`, `` ...).
    module_vis: HashMap<String, String>,
    /// Symbol -> (re-exporting module, `use` visibility) of each re-export.
    uses: HashMap<usize, Vec<(String, String)>>,
}

/// `pub mod x` -> `pub`, `mod x` -> ``.
fn mod_vis(sig: &str) -> String {
    sig.split(" mod ")
        .next()
        .filter(|v| v.starts_with("pub"))
        .unwrap_or("")
        .to_string()
}

fn parent(id: &str) -> Option<&str> {
    id.rsplit_once("::").map(|(p, _)| p)
}

fn root(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

/// The wider of two scopes (an ancestor wins; otherwise their common ancestor).
fn wider(a: String, b: String) -> String {
    let (pa, pb): (Vec<&str>, Vec<&str>) = (a.split("::").collect(), b.split("::").collect());
    let n = pa.iter().zip(&pb).take_while(|(x, y)| x == y).count();
    pa[..n.max(1)].join("::")
}

impl Reach {
    pub fn new(g: &Graph, ids: &ModuleIds, crate_ids: &[String]) -> Self {
        // Library roots: the crate id is the package name (no `/kind/target`).
        let mut modules: HashSet<String> = ids
            .values()
            .filter(|id| crate_ids.contains(id) && !id.contains('/'))
            .cloned()
            .collect();
        let module_vis: HashMap<String, String> = ids
            .iter()
            .map(|(&s, id)| (id.clone(), mod_vis(&g.symbols[s].signature)))
            .collect();
        // (re-exporting module, use visibility, target symbol) of every `pub use` path end.
        let all: Vec<(&String, &str, usize)> = g
            .occurrences
            .iter()
            .filter(|o| o.import && o.path_end && o.roles & 1 == 0)
            .filter_map(|o| {
                let vis = g.use_items[o.use_item?].vis.as_str();
                let from = match o.container {
                    Some(c) => module_of(g, ids, c),
                    None => g.files[o.file].module.and_then(|m| ids.get(&m)),
                }?;
                (!vis.is_empty()).then_some((from, vis, o.symbol))
            })
            .collect();
        let mut reexported = HashSet::new();
        loop {
            let before = (modules.len(), reexported.len());
            for (id, vis) in &module_vis {
                if vis == "pub" && parent(id).is_some_and(|p| modules.contains(p)) {
                    modules.insert(id.clone());
                }
            }
            for &(from, vis, sym) in &all {
                if vis != "pub" || !modules.contains(from) {
                    continue;
                }
                reexported.insert(sym);
                if let Some(id) = ids.get(&sym) {
                    modules.insert(id.clone());
                }
            }
            if (modules.len(), reexported.len()) == before {
                break;
            }
        }
        let reexported_names = reexported
            .iter()
            .filter_map(|&s| Some((module_of(g, ids, s)?.clone(), g.symbols[s].name.clone())))
            .collect();
        let mut uses: HashMap<usize, Vec<(String, String)>> = HashMap::new();
        for &(from, vis, sym) in &all {
            uses.entry(sym)
                .or_default()
                .push((from.clone(), vis.to_string()));
        }
        Reach {
            modules,
            reexported,
            reexported_names,
            module_vis,
            uses,
        }
    }

    /// A `pub` free item in `module`.
    pub fn item(&self, sym: usize, module: &str) -> bool {
        self.modules.contains(module) || self.reexported.contains(&sym)
    }

    /// A `pub` associated item of type `ty` defined in `module`: the type is reachable.
    pub fn assoc(&self, module: &str, ty: &str) -> bool {
        self.modules.contains(module)
            || self
                .reexported_names
                .contains(&(module.to_string(), ty.to_string()))
    }

    /// The module whose subtree can name a module itself.
    fn module_scope(&self, id: &str) -> String {
        let Some(p) = parent(id) else {
            return id.to_string();
        };
        match self.module_vis.get(id).map(String::as_str).unwrap_or("") {
            "pub" => self.module_scope(p),
            "" => p.to_string(),
            "pub(super)" => parent(p).unwrap_or(p).to_string(),
            "pub(self)" => p.to_string(),
            _ => root(id).to_string(),
        }
    }

    /// The scope of an item with visibility `vis` in `module`, widened by re-exports of `sym`.
    pub fn visible_in(&self, sym: usize, module: &str, vis: &str) -> String {
        let own = match vis {
            "pub" => self.module_scope(module),
            _ => root(module).to_string(),
        };
        self.uses
            .get(&sym)
            .into_iter()
            .flatten()
            .fold(own, |acc, (from, uv)| {
                let s = match uv.as_str() {
                    "pub" => self.module_scope(from),
                    "pub(super)" => parent(from).unwrap_or(from).to_string(),
                    _ => root(from).to_string(),
                };
                wider(acc, s)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scopes() {
        assert_eq!(mod_vis("pub mod a"), "pub");
        assert_eq!(mod_vis("pub(crate) mod a"), "pub(crate)");
        assert_eq!(mod_vis("mod a"), "");
        assert_eq!(wider("c::a::b".into(), "c::a".into()), "c::a");
        assert_eq!(wider("c::a::b".into(), "c::x".into()), "c");
        let r = Reach {
            modules: HashSet::new(),
            reexported: HashSet::new(),
            reexported_names: HashSet::new(),
            module_vis: [
                ("c::run".to_string(), "".to_string()),
                ("c::run::items".to_string(), "".to_string()),
                ("c::pubm".to_string(), "pub".to_string()),
            ]
            .into(),
            uses: [(7, vec![("c::run".to_string(), "pub(crate)".to_string())])].into(),
        };
        // `pub fn` in a private module of `run` is visible only inside `run`.
        assert_eq!(r.visible_in(1, "c::run::items", "pub"), "c::run");
        assert_eq!(r.visible_in(1, "c::pubm", "pub"), "c");
        assert_eq!(r.visible_in(1, "c::run::items", "pub(crate)"), "c");
        // A `pub(crate) use` re-export widens to the crate.
        assert_eq!(r.visible_in(7, "c::run::items", "pub"), "c");
    }
}
