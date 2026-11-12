//! Where items can be named from. A library crate exposes its root, `pub mod` children of
//! public modules and the targets of plain `pub use` items in public modules (globs
//! included). Inside a crate, an item is visible in the subtree of a module derived from its
//! own visibility, the visibility of the modules around it and `pub use` re-exports.

use crate::scip::Graph;
use crate::scip::modules::{ModuleIds, from_module, module_of};
use std::collections::{HashMap, HashSet};

pub struct Reach {
    /// Public module ids.
    modules: HashSet<String>,
    /// Symbols re-exported by a plain `pub use` in a public module; includes module symbols.
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

/// One `use` path end: `sym` is imported into module `from` with visibility `vis` (never empty).
struct PubUse<'a> {
    from: &'a String,
    vis: &'a str,
    sym: usize,
}

/// Every non-private `use` path end of the graph, in occurrence order. A use in a file outside
/// the module tree (no container module and no file module) is skipped on purpose: there is no
/// module to re-export from.
fn pub_uses<'a>(g: &'a Graph, ids: &'a ModuleIds) -> Vec<PubUse<'a>> {
    let end_of_import = |o: &&crate::scip::OccRow| o.import && o.path_end && !o.is_def();
    g.occurrences
        .iter()
        .filter(end_of_import)
        .filter_map(|o| {
            let vis = g.use_items[o.use_item?].vis.as_str();
            let from = from_module(g, ids, o)?;
            (!vis.is_empty()).then_some(PubUse {
                from,
                vis,
                sym: o.symbol,
            })
        })
        .collect()
}

/// Symbol -> (importing module, use visibility) of each of its uses, in `uses` order.
fn uses_by_symbol(uses: &[PubUse]) -> HashMap<usize, Vec<(String, String)>> {
    let mut by_sym: HashMap<usize, Vec<(String, String)>> = HashMap::new();
    for u in uses {
        by_sym
            .entry(u.sym)
            .or_default()
            .push((u.from.clone(), u.vis.to_string()));
    }
    by_sym
}

/// The public modules and re-exported symbols reachable from `modules` (the public roots).
struct Public {
    modules: HashSet<String>,
    reexported: HashSet<usize>,
}

impl Public {
    /// One pass of the fixpoint: a `pub mod` of a public module is public; a plain `pub use` in a
    /// public module re-exports its symbol, and a re-exported module is public too. A private
    /// module stops the walk. Whether anything was added.
    fn grow(
        &mut self,
        module_vis: &HashMap<String, String>,
        uses: &[PubUse],
        ids: &ModuleIds,
    ) -> bool {
        let before = (self.modules.len(), self.reexported.len());
        for (id, vis) in module_vis {
            if vis == "pub" && parent(id).is_some_and(|p| self.modules.contains(p)) {
                self.modules.insert(id.clone());
            }
        }
        for u in uses.iter().filter(|u| u.vis == "pub") {
            if self.modules.contains(u.from) {
                self.reexported.insert(u.sym);
                self.modules.extend(ids.get(&u.sym).cloned());
            }
        }
        (self.modules.len(), self.reexported.len()) != before
    }
}

impl Reach {
    pub fn new(g: &Graph, ids: &ModuleIds, crate_ids: &[String]) -> Self {
        // Library roots: the crate id is the package name (no `/kind/target`).
        let roots = ids
            .values()
            .filter(|id| crate_ids.contains(id) && !id.contains('/'))
            .cloned()
            .collect();
        let module_vis: HashMap<String, String> = ids
            .iter()
            .map(|(&s, id)| (id.clone(), mod_vis(&g.symbols[s].signature)))
            .collect();
        let all = pub_uses(g, ids);
        let mut public = Public {
            modules: roots,
            reexported: HashSet::new(),
        };
        while public.grow(&module_vis, &all, ids) {}
        let Public {
            modules,
            reexported,
        } = public;
        let reexported_names = reexported
            .iter()
            .filter_map(|&s| Some((module_of(g, ids, s)?.clone(), g.symbols[s].name.clone())))
            .collect();
        Reach {
            modules,
            reexported,
            reexported_names,
            module_vis,
            uses: uses_by_symbol(&all),
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

    /// The module whose subtree can name a module itself. `pub(in ..)`, `pub(crate)` and any
    /// other unknown visibility deliberately widen to the crate root (conservative: never
    /// narrower than the truth); `pub(self)` and `pub(super)` have their own arms.
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
    /// Only `pub` is narrowed (by the module chain); every other item visibility, `pub(in ..)`,
    /// `pub(self)` and unknown forms included, widens to the crate root on purpose (conservative).
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
    use crate::scip::OccRow;
    use crate::scip::testutil::Builder;

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

    /// Everything `Reach::new` derived, sorted for a stable comparison.
    fn dump(r: &Reach) -> String {
        let sorted = |v: Vec<String>| {
            let mut v = v;
            v.sort();
            v.join(", ")
        };
        let mut o = format!("modules: {}\n", sorted(r.modules.iter().cloned().collect()));
        o += &format!(
            "reexported: {}\n",
            sorted(r.reexported.iter().map(|s| s.to_string()).collect())
        );
        let names = r
            .reexported_names
            .iter()
            .map(|(m, n)| format!("{m}.{n}"))
            .collect();
        o += &format!("names: {}\n", sorted(names));
        o += &format!(
            "vis: {}\n",
            sorted(
                r.module_vis
                    .iter()
                    .map(|(m, v)| format!("{m}={v:?}"))
                    .collect()
            )
        );
        let uses = r.uses.iter().map(|(s, u)| format!("{s}:{u:?}")).collect();
        o + &format!("uses: {}\n", sorted(uses))
    }

    #[test]
    fn new_pub_chains_reexports_and_private_stops() {
        let mut b = Builder::default();
        let f = |b: &mut Builder, p: &str| b.file(p);
        let (lib, api, inner, hid, deep, leaf, pm, pq, ext) = (
            f(&mut b, "src/lib.rs"),
            f(&mut b, "src/api.rs"),
            f(&mut b, "src/api/inner.rs"),
            f(&mut b, "src/hidden.rs"),
            f(&mut b, "src/hidden/deep.rs"),
            f(&mut b, "src/hidden/deep/leaf.rs"),
            f(&mut b, "src/p.rs"),
            f(&mut b, "src/p/q.rs"),
            f(&mut b, "src/ext.rs"),
        );
        let root = b.module(lib, "c", "", None);
        let m_api = b.module(api, "c::api", "pub mod api", Some(root));
        b.module(inner, "c::api::inner", "pub mod inner", Some(m_api));
        let m_hid = b.module(hid, "c::hidden", "mod hidden", Some(root));
        let m_deep = b.module(deep, "c::hidden::deep", "pub mod deep", Some(m_hid));
        b.module(leaf, "c::hidden::deep::leaf", "pub mod leaf", Some(m_deep));
        let m_p = b.module(pm, "c::p", "mod p", Some(root));
        b.module(pq, "c::p::q", "pub mod q", Some(m_p));
        let m_ext = b.module(ext, "c::ext", "mod ext", Some(root));
        let thing = b.item(ext, "Thing", m_ext);
        let item2 = b.item(pm, "Item2", m_p);
        let (public, krate, private) =
            (b.use_item("pub"), b.use_item("pub(crate)"), b.use_item(""));
        let imp = |b: &Builder, file, sym, u, col| OccRow {
            import: true,
            use_item: Some(u),
            ..b.occ(file, sym, 0, col)
        };
        let occs = vec![
            // `pub use ext::Thing` in a public module: re-exported symbol and name.
            imp(&b, api, thing, public, 0),
            // `pub use hidden::deep` extends the public modules; `leaf` follows.
            imp(&b, api, m_deep, public, 1),
            // `pub use` in a private module: recorded as a use only.
            imp(&b, hid, item2, public, 2),
            // `pub(crate) use` and a private `use`: crate-wide use, then dropped.
            imp(&b, api, item2, krate, 3),
            imp(&b, api, thing, private, 4),
            // A plain reference, a definition and a mid-path segment are not re-exports.
            b.occ(api, thing, 1, 0),
            OccRow {
                roles: 1,
                ..imp(&b, api, thing, public, 5)
            },
            OccRow {
                path_end: false,
                ..imp(&b, api, thing, public, 6)
            },
        ];
        b.g.occurrences = occs;
        let r = Reach::new(&b.g, &b.ids, &["c".to_string()]);
        assert_eq!(dump(&r), EXPECTED.trim_start());
        assert!(r.item(thing, "c::ext"));
        assert!(!r.item(item2, "c::p"));
        assert!(r.assoc("c::ext", "Thing") && !r.assoc("c::ext", "Other"));
        assert!(r.item(0, "c::hidden::deep::leaf") && !r.item(0, "c::p::q"));
    }

    const EXPECTED: &str = r#"
modules: c, c::api, c::api::inner, c::hidden::deep, c::hidden::deep::leaf
reexported: 4, 9
names: c::ext.Thing, c::hidden::deep.deep
vis: c::api::inner="pub", c::api="pub", c::ext="", c::hidden::deep::leaf="pub", c::hidden::deep="pub", c::hidden="", c::p::q="pub", c::p="", c=""
uses: 10:[("c::hidden", "pub"), ("c::api", "pub(crate)")], 4:[("c::api", "pub")], 9:[("c::api", "pub")]
"#;

    fn vis_map(v: &[(&str, &str)]) -> HashMap<String, String> {
        v.iter()
            .map(|(m, s)| (m.to_string(), s.to_string()))
            .collect()
    }

    fn set(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn pub_uses_keeps_non_private_path_ends_only() {
        let mut b = Builder::default();
        let (lib, other) = (b.file("src/lib.rs"), b.file("src/other.rs"));
        let root = b.module(lib, "c", "", None);
        let m = b.module(other, "c::m", "mod m", Some(root));
        let x = b.item(other, "X", m);
        let (public, krate, private) =
            (b.use_item("pub"), b.use_item("pub(crate)"), b.use_item(""));
        let imp = |b: &Builder, u, col| OccRow {
            import: true,
            use_item: Some(u),
            ..b.occ(lib, x, 0, col)
        };
        b.g.occurrences = vec![
            imp(&b, public, 0),
            imp(&b, krate, 1),
            imp(&b, private, 2),
            // Not a use: reference, definition, mid-path, no use item.
            b.occ(lib, x, 1, 0),
            OccRow {
                roles: 1,
                ..imp(&b, public, 3)
            },
            OccRow {
                path_end: false,
                ..imp(&b, public, 4)
            },
            OccRow {
                use_item: None,
                ..imp(&b, public, 5)
            },
            // Module from the container when there is one.
            OccRow {
                container: Some(m),
                ..imp(&b, public, 6)
            },
        ];
        let got: Vec<(&str, &str, usize)> = pub_uses(&b.g, &b.ids)
            .iter()
            .map(|u| (u.from.as_str(), u.vis, u.sym))
            .collect();
        assert_eq!(
            got,
            [("c", "pub", x), ("c", "pub(crate)", x), ("c::m", "pub", x)]
        );
    }

    #[test]
    fn pub_uses_skips_files_outside_the_module_tree_on_purpose_and_empty_graph() {
        let mut b = Builder::default();
        assert!(pub_uses(&b.g, &b.ids).is_empty());
        let f = b.file("src/x.rs");
        let m = b.module(f, "c", "", None);
        let public = b.use_item("pub");
        let mut o = b.occ(f, m, 0, 0);
        (o.import, o.use_item) = (true, Some(public));
        b.g.occurrences = vec![o];
        b.g.files[f].module = None;
        assert!(pub_uses(&b.g, &b.ids).is_empty());
    }

    #[test]
    fn module_scope_widens_pub_in_to_crate() {
        let r = Reach {
            modules: HashSet::new(),
            reexported: HashSet::new(),
            reexported_names: HashSet::new(),
            module_vis: vis_map(&[
                ("c::a::b", "pub(in crate::a)"),
                ("c::a::s", "pub(self)"),
                ("c::a::c", "pub(super)"),
            ]),
            uses: HashMap::new(),
        };
        // Deliberate: `pub(in ..)` is not parsed, so it is crate-wide, never narrower.
        assert_eq!(r.module_scope("c::a::b"), "c");
        // `pub(self)` and `pub(super)` modules have their own arms.
        assert_eq!(r.module_scope("c::a::s"), "c::a");
        assert_eq!(r.module_scope("c::a::c"), "c");
        // Item visibility other than `pub` is crate-wide, `pub(self)` included.
        assert_eq!(r.visible_in(1, "c::a::b", "pub(in crate::a)"), "c");
        assert_eq!(r.visible_in(1, "c::a::b", "pub(self)"), "c");
    }

    #[test]
    fn uses_by_symbol_groups_in_order() {
        assert!(uses_by_symbol(&[]).is_empty());
        let (a, c) = ("c::a".to_string(), "c".to_string());
        let uses = [
            PubUse {
                from: &a,
                vis: "pub",
                sym: 1,
            },
            PubUse {
                from: &c,
                vis: "pub(crate)",
                sym: 2,
            },
            PubUse {
                from: &c,
                vis: "pub(super)",
                sym: 1,
            },
        ];
        let got = uses_by_symbol(&uses);
        assert_eq!(got.len(), 2);
        assert_eq!(
            got[&1],
            [
                ("c::a".to_string(), "pub".to_string()),
                ("c".to_string(), "pub(super)".to_string())
            ]
        );
        assert_eq!(got[&2], [("c".to_string(), "pub(crate)".to_string())]);
    }

    /// `grow` until it adds nothing (what `Reach::new` does).
    fn closure(
        modules: HashSet<String>,
        vis: &HashMap<String, String>,
        uses: &[PubUse],
        ids: &ModuleIds,
    ) -> Public {
        let mut p = Public {
            modules,
            reexported: HashSet::new(),
        };
        while p.grow(vis, uses, ids) {}
        p
    }

    #[test]
    fn closure_empty_graph_keeps_roots() {
        let p = closure(set(&["c"]), &HashMap::new(), &[], &ModuleIds::new());
        assert_eq!(p.modules, set(&["c"]));
        assert!(p.reexported.is_empty());
    }

    #[test]
    fn closure_follows_pub_mod_chain_and_stops_at_private() {
        let vis = vis_map(&[
            ("c", ""),
            ("c::a", "pub"),
            ("c::a::b", "pub"),
            ("c::p", ""),
            ("c::p::q", "pub"),
            ("c::r", "pub(crate)"),
        ]);
        let p = closure(set(&["c"]), &vis, &[], &ModuleIds::new());
        assert_eq!(p.modules, set(&["c", "c::a", "c::a::b"]));
    }

    #[test]
    fn closure_reexport_extends_public_modules() {
        let vis = vis_map(&[("c::p", ""), ("c::p::q", "pub"), ("c::api", "pub")]);
        let ids: ModuleIds = [(5, "c::p".to_string()), (6, "c::api".to_string())].into();
        let (c, api, other) = ("c".to_string(), "c::api".to_string(), "c::hid".to_string());
        let uses = [
            // In a private module: ignored. pub(crate): ignored. Plain item: no module added.
            PubUse {
                from: &other,
                vis: "pub",
                sym: 9,
            },
            PubUse {
                from: &c,
                vis: "pub(crate)",
                sym: 8,
            },
            PubUse {
                from: &api,
                vis: "pub",
                sym: 7,
            },
            // Re-exporting the private module makes it, then its `pub` child, public.
            PubUse {
                from: &c,
                vis: "pub",
                sym: 5,
            },
        ];
        let p = closure(set(&["c"]), &vis, &uses, &ids);
        assert_eq!(p.modules, set(&["c", "c::api", "c::p", "c::p::q"]));
        assert_eq!(p.reexported, [5, 7].into());
    }
}
