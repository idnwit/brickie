//! The cargo layout an index is normalised against: package directories and target roots.
//! `run` fills it from the workspace metadata.

pub struct Package {
    /// Package directory relative to the workspace root (`""` for the root).
    pub dir: String,
}

pub struct Target {
    pub package: usize,
    pub name: String,
    /// Root source file relative to the workspace root.
    pub src_path: String,
}

pub struct Layout {
    pub packages: Vec<Package>,
    pub targets: Vec<Target>,
}
