//! `[measure]` in the root `brickie.toml`: what `brickie measure` indexes besides the host.

use anyhow::{Result, bail};
use serde::Deserialize;

/// The `[measure]` table. The host target and all cargo features are always measured.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasureCfg {
    /// Target triples measured in addition to the host, for code under `cfg(target_*)`.
    #[serde(default)]
    pub additional_targets: Vec<String>,
}

impl MeasureCfg {
    /// Trim, check and dedupe the target names; the result is sorted.
    pub(super) fn normalise(&mut self) -> Result<()> {
        for t in &mut self.additional_targets {
            *t = t.trim().to_string();
            let ok = !t.is_empty()
                && t.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
            if !ok {
                bail!(
                    "[measure] additional_targets: '{t}' is not a target triple (for example \"wasm32-unknown-unknown\")"
                );
            }
        }
        self.additional_targets.sort();
        self.additional_targets.dedup();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::load_measure;
    use super::*;

    fn load_str(text: &str) -> Result<MeasureCfg> {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("brickie.toml");
        std::fs::write(&p, text).unwrap();
        load_measure(&p)
    }

    #[test]
    fn parses_dedupes_and_rejects() {
        let c = load_str("[measure]\nadditional_targets=[\"b-x\", \"a-y\", \"b-x\"]\n").unwrap();
        assert_eq!(c.additional_targets, ["a-y", "b-x"]);
        assert!(
            load_str("[[brick]]\nname=\"a\"\n")
                .unwrap()
                .additional_targets
                .is_empty()
        );
        assert!(load_str("[measure]\nfeatures=\"all\"\n").is_err());
        assert!(load_str("[measure]\nadditional_targets=[\"bad target\"]\n").is_err());
        assert!(load_str("[measure]\nadditional_targets=[\"\"]\n").is_err());
        assert!(load_str("[measure]\nadditional_targets=\"x\"\n").is_err());
    }

    #[test]
    fn full_loader_sees_measure_and_only_at_the_root() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("brickie.toml");
        std::fs::write(
            &root,
            "[measure]\nadditional_targets=[\"wasm32-unknown-unknown\"]\n[[brick]]\nname=\"w\"\npath=\"w\"\n",
        )
        .unwrap();
        std::fs::create_dir(dir.path().join("w")).unwrap();
        std::fs::write(dir.path().join("w/brickie.toml"), "").unwrap();
        let c = super::super::load(&root).unwrap();
        assert_eq!(c.measure.additional_targets, ["wasm32-unknown-unknown"]);
        std::fs::write(
            dir.path().join("w/brickie.toml"),
            "[measure]\nadditional_targets=[\"x\"]\n",
        )
        .unwrap();
        let e = format!("{:#}", super::super::load(&root).unwrap_err());
        assert!(e.contains("root brickie.toml"), "{e}");
    }
}
