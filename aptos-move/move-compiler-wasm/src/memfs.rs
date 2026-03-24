// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! In-memory filesystem for WASM environments
//!
//! Provides a global in-memory filesystem that replaces std::fs operations
//! This allows the compiler to work in browsers without any filesystem

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Global in-memory filesystem
static MEMFS: Mutex<Option<MemoryFileSystem>> = Mutex::new(None);

/// Initialize the in-memory filesystem
pub fn init() {
    let mut guard = MEMFS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(MemoryFileSystem::new());
    }
}

/// Write a file to the in-memory filesystem
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let mut guard = MEMFS.lock().unwrap();
    let fs = guard.as_mut().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "MemFS not initialized")
    })?;

    let path = path.as_ref().to_path_buf();
    fs.files.insert(path, contents.as_ref().to_vec());
    Ok(())
}

/// Read a file from the in-memory filesystem
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let guard = MEMFS.lock().unwrap();
    let fs = guard.as_ref().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "MemFS not initialized")
    })?;

    let path = path.as_ref();
    fs.files.get(path).cloned().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, format!("File not found: {:?}", path))
    })
}

/// Remove a file from the in-memory filesystem
pub fn remove<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let mut guard = MEMFS.lock().unwrap();
    let fs = guard.as_mut().ok_or_else(|| {
        io::Error::new(io::ErrorKind::Other, "MemFS not initialized")
    })?;

    let path = path.as_ref();
    fs.files.remove(path);
    Ok(())
}

/// Check if a file exists in the in-memory filesystem
pub fn exists<P: AsRef<Path>>(path: P) -> bool {
    let guard = MEMFS.lock().unwrap();
    if let Some(fs) = guard.as_ref() {
        fs.files.contains_key(path.as_ref())
    } else {
        false
    }
}

/// List all files in the in-memory filesystem
pub fn list() -> Vec<PathBuf> {
    let guard = MEMFS.lock().unwrap();
    if let Some(fs) = guard.as_ref() {
        fs.files.keys().cloned().collect()
    } else {
        vec![]
    }
}

/// Clear the in-memory filesystem
pub fn clear() {
    let mut guard = MEMFS.lock().unwrap();
    if let Some(fs) = guard.as_mut() {
        fs.files.clear();
    }
}

struct MemoryFileSystem {
    files: HashMap<PathBuf, Vec<u8>>,
}

impl MemoryFileSystem {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memfs() {
        init();

        let path = "/tmp/test.txt";
        let content = b"hello world";

        write(path, content).unwrap();
        assert!(exists(path));

        let read_content = read(path).unwrap();
        assert_eq!(read_content, content);

        remove(path).unwrap();
        assert!(!exists(path));
    }
}
