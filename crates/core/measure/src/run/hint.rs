//! The hint for code under `cfg(target_arch = ..)` / `cfg(target_os = ..)` that no measured
//! target covers. A text scan: cheap, and a false positive only costs one line of output.

use std::collections::BTreeSet;

/// The `(key, value)` pairs of `target_arch = "x"` / `target_os = "x"` in `src`, leaving out
/// comment lines and values directly under `not(` (that code is built for the other targets).
pub fn target_cfgs(src: &str) -> BTreeSet<(&'static str, String)> {
    let mut out = BTreeSet::new();
    for line in src.lines().filter(|l| !l.trim_start().starts_with("//")) {
        for key in ["target_arch", "target_os"] {
            for (at, _) in line.match_indices(key) {
                if line[..at].trim_end().ends_with("not(") {
                    continue;
                }
                let rest = line[at + key.len()..].trim_start();
                let Some(rest) = rest.strip_prefix('=') else {
                    continue;
                };
                let Some(rest) = rest.trim_start().strip_prefix('"') else {
                    continue;
                };
                if let Some(end) = rest.find('"') {
                    out.insert((key, rest[..end].to_string()));
                }
            }
        }
    }
    out
}

/// The triple to suggest for a cfg value, when there is an obvious one.
fn suggestion(key: &str, value: &str) -> Option<&'static str> {
    Some(match (key, value) {
        ("target_arch", "wasm32") => "wasm32-unknown-unknown",
        ("target_arch", "aarch64") => "aarch64-unknown-linux-gnu",
        ("target_arch", "x86_64") => "x86_64-unknown-linux-gnu",
        ("target_os", "windows") => "x86_64-pc-windows-msvc",
        ("target_os", "macos") => "aarch64-apple-darwin",
        ("target_os", "linux") => "x86_64-unknown-linux-gnu",
        ("target_os", "android") => "aarch64-linux-android",
        ("target_os", "ios") => "aarch64-apple-ios",
        ("target_os", "wasi") => "wasm32-wasip1",
        _ => return None,
    })
}

/// Whether `triple` is on the cfg `value` family.
fn covers(triple: &str, key: &str, value: &str) -> bool {
    let parts: Vec<&str> = triple.split('-').collect();
    if key == "target_arch" {
        return parts.first() == Some(&value);
    }
    let os = if value == "macos" { "darwin" } else { value };
    parts.iter().skip(1).any(|p| p.starts_with(os))
}

/// One hint line per uncovered cfg family found in `cfgs`; `triples` are the host and the
/// additional targets. Values without an obvious triple are ignored.
pub fn hints(cfgs: &BTreeSet<(&'static str, String)>, triples: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    for (key, value) in cfgs {
        if triples.iter().any(|t| covers(t, key, value)) {
            continue;
        }
        let Some(triple) = suggestion(key, value) else {
            continue;
        };
        if lines.iter().any(|(t, _): &(&str, String)| *t == triple) {
            continue;
        }
        lines.push((
            triple,
            format!(
                "hint: code under cfg({key} = \"{value}\") is not measured; add \"{triple}\" to [measure] additional_targets"
            ),
        ));
    }
    lines.into_iter().map(|(_, l)| l).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: &str = "x86_64-unknown-linux-gnu";

    #[test]
    fn finds_cfg_values() {
        let c = target_cfgs(
            "#[cfg(target_arch = \"wasm32\")]\n#[cfg(target_os=\"windows\")]\nlet target_arch = 1;\n#[cfg(not(target_os = \"macos\"))]\n// cfg(target_os = \"ios\")",
        );
        assert!(c.contains(&("target_arch", "wasm32".into())));
        assert!(c.contains(&("target_os", "windows".into())));
        assert_eq!(c.len(), 2, "{c:?}");
    }

    #[test]
    fn hints_only_for_uncovered_families() {
        let c = target_cfgs(
            "cfg(target_arch = \"wasm32\") cfg(target_os = \"linux\") cfg(target_arch = \"x86_64\")",
        );
        let h = hints(&c, &[HOST.into()]);
        assert_eq!(
            h,
            [
                "hint: code under cfg(target_arch = \"wasm32\") is not measured; add \"wasm32-unknown-unknown\" to [measure] additional_targets"
            ]
        );
        let both = [HOST.into(), "wasm32-unknown-unknown".into()];
        assert!(hints(&c, &both).is_empty());
        let mac = target_cfgs("cfg(target_os = \"macos\")");
        assert!(hints(&mac, &["aarch64-apple-darwin".into()]).is_empty());
        assert_eq!(hints(&mac, &[HOST.into()]).len(), 1);
    }
}
