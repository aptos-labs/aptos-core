// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! In-memory source file management for filesystem-free compilation
//!
//! This module provides [`SourceMap`], which allows compiling Move code from
//! in-memory strings instead of requiring filesystem access. This enables:
//! - WASM/browser deployment
//! - Testing without temporary files
//! - IDE language server integration
//! - Programmatic code generation and compilation
//!
//! # Example
//! ```
//! use move_compiler_v2::sources::SourceMap;
//!
//! let mut sources = SourceMap::new();
//! sources.add_file("MyModule.move", r#"
//!     module 0x1::MyModule {
//!         public fun hello(): u64 { 42 }
//!     }
//! "#);
//! ```

use move_command_line_common::files::FileHash;
use move_symbol_pool::Symbol;
use std::collections::{BTreeMap, HashMap};

/// Type alias for the internal source text format used by the legacy compiler
/// Note: Uses HashMap to match legacy_move_compiler::diagnostics::FilesSourceText
pub type FilesSourceText = HashMap<FileHash, (Symbol, String)>;

/// In-memory collection of Move source files
///
/// `SourceMap` provides a simple interface for managing source code without
/// requiring filesystem access. Files are identified by virtual paths (used
/// in error messages) and contain Move source code as strings.
///
/// # Virtual Paths
///
/// Virtual paths should be meaningful for error reporting:
/// - ✅ Good: `"MyModule.move"`, `"src/token.move"`
/// - ❌ Bad: `"3a7f5c9e.move"`, `"temp.move"`
///
/// # Example
/// ```
/// use move_compiler_v2::sources::SourceMap;
///
/// let mut sources = SourceMap::new();
/// sources.add_file("Example.move", r#"
///     module 0x42::Example {
///         public fun example(): u64 { 42 }
///     }
/// "#);
///
/// assert_eq!(sources.len(), 1);
/// assert!(sources.contains(&"Example.move".into()));
/// ```
#[derive(Clone, Debug, Default)]
pub struct SourceMap {
    /// Map from virtual file path to source code content
    /// Using BTreeMap for deterministic ordering (important for reproducible builds)
    files: BTreeMap<Symbol, String>,
}

impl SourceMap {
    /// Create a new empty source map
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    ///
    /// let sources = SourceMap::new();
    /// assert_eq!(sources.len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    /// Add a source file with virtual path
    ///
    /// The virtual path is used in error messages, so it should be meaningful
    /// and descriptive. The content should be valid Move source code.
    ///
    /// # Arguments
    /// * `path` - Virtual file path (e.g., "MyModule.move")
    /// * `content` - Move source code as string
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// sources.add_file("test.move", r#"
    ///     module 0x1::Test {
    ///         public fun hello(): u64 { 42 }
    ///     }
    /// "#);
    /// ```
    pub fn add_file(&mut self, path: impl Into<Symbol>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Get source content for a virtual path
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    /// use move_symbol_pool::Symbol;
    ///
    /// let mut sources = SourceMap::new();
    /// sources.add_file("test.move", "module 0x1::Test {}");
    ///
    /// let path = Symbol::from("test.move");
    /// assert!(sources.get_file(&path).is_some());
    /// ```
    pub fn get_file(&self, path: &Symbol) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    /// Check if a file exists in the source map
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    /// use move_symbol_pool::Symbol;
    ///
    /// let mut sources = SourceMap::new();
    /// sources.add_file("test.move", "module 0x1::Test {}");
    ///
    /// assert!(sources.contains(&Symbol::from("test.move")));
    /// assert!(!sources.contains(&Symbol::from("missing.move")));
    /// ```
    pub fn contains(&self, path: &Symbol) -> bool {
        self.files.contains_key(path)
    }

    /// Get an iterator over all virtual file paths
    ///
    /// Paths are returned in sorted order (BTreeMap ordering).
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// sources.add_file("a.move", "module 0x1::A {}");
    /// sources.add_file("b.move", "module 0x1::B {}");
    ///
    /// let paths: Vec<_> = sources.paths().map(|s| s.as_str()).collect();
    /// assert_eq!(paths, vec!["a.move", "b.move"]);
    /// ```
    pub fn paths(&self) -> impl Iterator<Item = &Symbol> {
        self.files.keys()
    }

    /// Get number of source files in the map
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    ///
    /// let mut sources = SourceMap::new();
    /// assert_eq!(sources.len(), 0);
    ///
    /// sources.add_file("test.move", "module 0x1::Test {}");
    /// assert_eq!(sources.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if the source map is empty
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    ///
    /// let sources = SourceMap::new();
    /// assert!(sources.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Convert to internal FilesSourceText format
    ///
    /// This creates a map keyed by content hash (SHA256) with values of
    /// (filename, content). The compiler uses this format internally for
    /// error reporting and source location tracking.
    ///
    /// This is an internal function used by the compiler pipeline.
    pub fn to_files_source_text(&self) -> FilesSourceText {
        self.files
            .iter()
            .map(|(name, content)| {
                let hash = FileHash::new(content);
                (hash, (*name, content.clone()))
            })
            .collect()
    }
}

impl FromIterator<(Symbol, String)> for SourceMap {
    /// Create a SourceMap from an iterator of (path, content) pairs
    ///
    /// # Example
    /// ```
    /// use move_compiler_v2::sources::SourceMap;
    /// use move_symbol_pool::Symbol;
    ///
    /// let files = vec![
    ///     (Symbol::from("a.move"), "module 0x1::A {}".to_string()),
    ///     (Symbol::from("b.move"), "module 0x1::B {}".to_string()),
    /// ];
    ///
    /// let sources: SourceMap = files.into_iter().collect();
    /// assert_eq!(sources.len(), 2);
    /// ```
    fn from_iter<T: IntoIterator<Item = (Symbol, String)>>(iter: T) -> Self {
        Self {
            files: iter.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_source_map() {
        let sources = SourceMap::new();
        assert_eq!(sources.len(), 0);
        assert!(sources.is_empty());
    }

    #[test]
    fn test_add_file() {
        let mut sources = SourceMap::new();
        sources.add_file("test.move", "module 0x1::Test {}");

        assert_eq!(sources.len(), 1);
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_get_file() {
        let mut sources = SourceMap::new();
        let content = "module 0x1::Test {}";
        sources.add_file("test.move", content);

        let path = Symbol::from("test.move");
        assert_eq!(sources.get_file(&path), Some(content));

        let missing = Symbol::from("missing.move");
        assert_eq!(sources.get_file(&missing), None);
    }

    #[test]
    fn test_contains() {
        let mut sources = SourceMap::new();
        sources.add_file("test.move", "module 0x1::Test {}");

        assert!(sources.contains(&Symbol::from("test.move")));
        assert!(!sources.contains(&Symbol::from("missing.move")));
    }

    #[test]
    fn test_paths_iterator() {
        let mut sources = SourceMap::new();
        sources.add_file("b.move", "module 0x1::B {}");
        sources.add_file("a.move", "module 0x1::A {}");
        sources.add_file("c.move", "module 0x1::C {}");

        // Should be in sorted order (BTreeMap)
        let paths: Vec<_> = sources.paths().map(|s| s.as_str()).collect();
        assert_eq!(paths, vec!["a.move", "b.move", "c.move"]);
    }

    #[test]
    fn test_from_iterator() {
        let files = vec![
            (Symbol::from("a.move"), "module 0x1::A {}".to_string()),
            (Symbol::from("b.move"), "module 0x1::B {}".to_string()),
        ];

        let sources: SourceMap = files.into_iter().collect();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains(&Symbol::from("a.move")));
        assert!(sources.contains(&Symbol::from("b.move")));
    }

    #[test]
    fn test_to_files_source_text() {
        let mut sources = SourceMap::new();
        sources.add_file("test.move", "module 0x1::Test {}");

        let files_source_text = sources.to_files_source_text();

        assert_eq!(files_source_text.len(), 1);

        // Should contain the file with correct content
        let (_, (name, content)) = files_source_text.iter().next().unwrap();
        assert_eq!(name.as_str(), "test.move");
        assert_eq!(content, "module 0x1::Test {}");
    }

    #[test]
    fn test_duplicate_paths_overwrite() {
        let mut sources = SourceMap::new();
        sources.add_file("test.move", "first content");
        sources.add_file("test.move", "second content");

        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources.get_file(&Symbol::from("test.move")),
            Some("second content")
        );
    }

    #[test]
    fn test_file_hash_different_content() {
        let mut sources1 = SourceMap::new();
        sources1.add_file("test.move", "content A");

        let mut sources2 = SourceMap::new();
        sources2.add_file("test.move", "content B");

        let files1 = sources1.to_files_source_text();
        let files2 = sources2.to_files_source_text();

        // Different content should produce different hashes
        let hash1 = files1.keys().next().unwrap();
        let hash2 = files2.keys().next().unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_file_hash_same_content() {
        let mut sources1 = SourceMap::new();
        sources1.add_file("test.move", "same content");

        let mut sources2 = SourceMap::new();
        sources2.add_file("other.move", "same content");

        let files1 = sources1.to_files_source_text();
        let files2 = sources2.to_files_source_text();

        // Same content should produce same hash even with different paths
        let hash1 = files1.keys().next().unwrap();
        let hash2 = files2.keys().next().unwrap();
        assert_eq!(hash1, hash2);
    }
}
