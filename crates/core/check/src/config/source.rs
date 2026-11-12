//! Where config files are read from: the disk, or a virtual file set (a change's preview).

use std::io;
use std::path::{Path, PathBuf};

/// The file operations `load` needs. `canonicalize` gives the identity of a file (include and
/// path cycles) and the base directory of `path:` members.
pub trait FileSource {
    fn read(&self, path: &Path) -> io::Result<String>;
    fn is_file(&self, path: &Path) -> bool;
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;
}

/// The real file system.
pub struct Disk;

impl FileSource for Disk {
    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }
}
