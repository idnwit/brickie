//! Surface uses: Rust lets a caller use a type without writing its name, so an item is also
//! used when it appears in the surface of a used item of its crate: the signature (parameters,
//! return type, generic bounds, aliased type), the `pub` fields and variants of a type, the
//! members of a trait, and the traits a type implements. The set is the fixpoint over the used
//! items, computed once per index. Matching is by word: an identifier of the surface text equal
//! to the name of a type, trait or constant of the same crate. It may over-count, which only
//! silences a warning.

use brickie_facts::{Cache, Item};
use std::collections::{HashMap, HashSet, VecDeque};

/// The used item whose surface names an item.
pub(super) struct Via<'a> {
    pub item: &'a Item,
    pub file: &'a str,
}

/// Item identity: defining module and symbol.
pub(super) type Id<'a> = (&'a str, &'a str);

pub(super) fn id(item: &Item) -> Id<'_> {
    (item.module.as_str(), item.symbol.as_str())
}

/// Item kinds a signature can name.
fn nameable(kind: &str) -> bool {
    matches!(
        kind,
        "struct" | "enum" | "union" | "trait" | "type" | "const"
    )
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
}

/// Every item reached through the surface of a seed (an item with a use of its own) or of an
/// item reached before, with the first item that reaches it.
pub(super) fn reach<'a>(
    cache: &'a Cache,
    crate_of: &HashMap<&'a str, &'a str>,
    is_seed: impl Fn(&Item) -> bool,
) -> HashMap<Id<'a>, Via<'a>> {
    let all: Vec<(&str, &Item)> = cache
        .files
        .iter()
        .flat_map(|(f, fd)| fd.items.iter().map(move |i| (f.as_str(), i)))
        .collect();
    let mut by_name: HashMap<(&str, &str), Vec<usize>> = HashMap::new();
    for (n, (_, i)) in all.iter().enumerate() {
        if let (true, Some(k)) = (nameable(&i.kind), crate_of.get(i.module.as_str())) {
            let name = i.name.rsplit("::").next().unwrap_or_default();
            by_name.entry((k, name)).or_default().push(n);
        }
    }
    let mut seen: HashSet<usize> = (0..all.len()).filter(|&n| is_seed(all[n].1)).collect();
    let mut queue: VecDeque<usize> = seen.iter().copied().collect();
    let mut via = HashMap::new();
    while let Some(n) = queue.pop_front() {
        let (file, y) = all[n];
        let Some(k) = crate_of.get(y.module.as_str()) else {
            continue;
        };
        let text = std::iter::once(&y.signature).chain(&y.surface);
        let named = text
            .flat_map(|t| words(t))
            .filter_map(|w| by_name.get(&(*k, w)))
            .flatten();
        for &m in named {
            if seen.insert(m) {
                via.insert(id(all[m].1), Via { item: y, file });
                queue.push_back(m);
            }
        }
    }
    via
}
