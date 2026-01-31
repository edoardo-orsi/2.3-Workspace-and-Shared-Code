use std::fs::read_dir;
use std::path::Path;

pub trait PathExt {
    /// Returns true if the path is a directory and contains at least one file.
    fn has_files(&self) -> bool;
}

impl PathExt for Path {
    fn has_files(&self) -> bool {
        read_dir(self)
            .map(|mut entries| {
                entries.any(|entry| entry.map(|e| e.path().is_file()).unwrap_or(false))
            })
            .unwrap_or(false)
    }
}
