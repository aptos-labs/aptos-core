// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Virtual Filesystem for WASM
//!
//! Provides an in-memory filesystem that works in browsers without std::fs

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// In-memory virtual filesystem for WASM
///
/// Since browsers don't have a filesystem, we maintain sources in memory
/// and provide them to the compiler without touching std::fs
#[derive(Clone, Debug)]
pub struct VirtualFS {
    files: Arc<RwLock<HashMap<PathBuf, Vec<u8>>>>,
}

impl VirtualFS {
    /// Create a new virtual filesystem
    pub fn new() -> Self {
        Self {
            files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a file to the virtual filesystem
    pub fn add_file<P: AsRef<Path>>(&self, path: P, content: Vec<u8>) {
        let path = path.as_ref().to_path_buf();
        self.files.write().unwrap().insert(path, content);
    }

    /// Add a text file to the virtual filesystem
    pub fn add_text_file<P: AsRef<Path>>(&self, path: P, content: &str) {
        self.add_file(path, content.as_bytes().to_vec());
    }

    /// Get a file from the virtual filesystem
    pub fn get_file<P: AsRef<Path>>(&self, path: P) -> Option<Vec<u8>> {
        let path = path.as_ref();
        self.files.read().unwrap().get(path).cloned()
    }

    /// Check if a file exists
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        self.files.read().unwrap().contains_key(path)
    }

    /// List all files
    pub fn list_files(&self) -> Vec<PathBuf> {
        self.files.read().unwrap().keys().cloned().collect()
    }

    /// Remove a file
    pub fn remove_file<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();
        self.files.write().unwrap().remove(path);
    }

    /// Clear all files
    pub fn clear(&self) {
        self.files.write().unwrap().clear();
    }
}

impl Default for VirtualFS {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_file() {
        let vfs = VirtualFS::new();
        vfs.add_text_file("test.move", "module test {}");

        assert!(vfs.exists("test.move"));
        assert_eq!(vfs.get_file("test.move"), Some(b"module test {}".to_vec()));
    }

    #[test]
    fn test_remove_file() {
        let vfs = VirtualFS::new();
        vfs.add_text_file("test.move", "module test {}");
        assert!(vfs.exists("test.move"));

        vfs.remove_file("test.move");
        assert!(!vfs.exists("test.move"));
    }

    #[test]
    fn test_list_files() {
        let vfs = VirtualFS::new();
        vfs.add_text_file("a.move", "a");
        vfs.add_text_file("b.move", "b");

        let files = vfs.list_files();
        assert_eq!(files.len(), 2);
    }
}
