//! Cargo metadata facts: workspace packages, their targets and declared dependencies.

use anyhow::{Context, Result};
use cargo_metadata::{MetadataCommand, PackageId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Package {
    pub name: String,
    pub version: String,
    /// Package directory relative to the workspace root (`""` for the root).
    pub dir: String,
}

pub struct Target {
    pub package: usize,
    pub name: String,
    pub kind: String,
    /// Root source file relative to the workspace root.
    pub src_path: String,
}

pub struct Workspace {
    pub root: PathBuf,
    pub packages: Vec<Package>,
    pub targets: Vec<Target>,
}

pub fn load(workspace: &Path) -> Result<Workspace> {
    let md = MetadataCommand::new()
        .manifest_path(workspace.join("Cargo.toml"))
        .exec()
        .context("cargo metadata failed")?;
    let root = PathBuf::from(md.workspace_root.as_str());
    let rel = |p: &Path| -> String {
        p.strip_prefix(&root)
            .unwrap_or(p)
            .to_string_lossy()
            .into_owned()
    };
    let by_id: HashMap<&PackageId, &cargo_metadata::Package> =
        md.packages.iter().map(|p| (&p.id, p)).collect();
    let mut index: HashMap<&PackageId, usize> = HashMap::new();
    let mut packages = Vec::new();
    let mut targets = Vec::new();
    for id in md.workspace_members.iter() {
        let p = by_id[id];
        let pi = packages.len();
        index.insert(id, pi);
        let manifest = rel(p.manifest_path.as_std_path());
        let dir = Path::new(&manifest)
            .parent()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default();
        packages.push(Package {
            name: p.name.to_string(),
            version: p.version.to_string(),
            dir,
        });
        for t in &p.targets {
            targets.push(Target {
                package: pi,
                name: t.name.clone(),
                kind: t
                    .kind
                    .iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
                src_path: rel(t.src_path.as_std_path()),
            });
        }
    }
    Ok(Workspace {
        root,
        packages,
        targets,
    })
}
