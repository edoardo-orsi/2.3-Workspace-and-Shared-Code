use std::fs;
use tempdir::TempDir;

// Helper to create temporary config file
pub fn create_temp_config(dir: &TempDir, name: &str, content: &str) {
    let path = dir.path().join(name);
    fs::write(path, content).unwrap();
}
