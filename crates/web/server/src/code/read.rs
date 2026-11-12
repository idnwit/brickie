//! A cached file's text: only files listed in the cache, regular files inside the workspace
//! root (no symlink out of it), of a reasonable size; `stale` when the hash differs from the
//! measured one.

use brickie_check::check::Loaded;
use codeview_port::FileText;
use std::path::Path;

/// Largest file served.
const MAX_BYTES: u64 = 4 * 1024 * 1024;

/// The text of `path` (a cache key), or `None` when it is not served.
pub fn file(l: &Loaded, path: &str) -> Option<FileText> {
    let fd = l.cache.files.get(path)?;
    let bytes = read_confined(Path::new(&l.cache.workspace_root), path)?;
    Some(FileText {
        path: path.to_string(),
        stale: brickie_facts::sha256_hex(&bytes) != fd.sha256,
        text: String::from_utf8_lossy(&bytes).into_owned(),
        lang: lang(path).to_string(),
    })
}

/// The bytes of `root/rel` when it is a regular file of at most [`MAX_BYTES`] whose canonical
/// path stays under the canonical root.
pub fn read_confined(root: &Path, rel: &str) -> Option<Vec<u8>> {
    let path = root.join(rel);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return None;
    }
    let real = path.canonicalize().ok()?;
    if !real.starts_with(root.canonicalize().ok()?) {
        return None;
    }
    std::fs::read(real).ok()
}

/// The Monaco language id of a path.
fn lang(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("rs") => "rust",
        _ => "plaintext",
    }
}
