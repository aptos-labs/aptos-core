// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Move package compilation support

use wasm_bindgen::prelude::*;
use std::collections::BTreeMap;

use crate::{CompilationResult, PackageMetadata};

/// In-memory Move package builder for WASM
///
/// This allows building Move packages entirely in memory without filesystem access.
/// JavaScript provides all source files and dependencies.
#[wasm_bindgen]
pub struct MovePackage {
    metadata: PackageMetadata,
    sources: BTreeMap<String, String>,
}

#[wasm_bindgen]
impl MovePackage {
    /// Create a new Move package
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, version: String) -> Self {
        Self {
            metadata: PackageMetadata::new(name, version),
            sources: BTreeMap::new(),
        }
    }

    /// Add a source file to the package
    ///
    /// # Arguments
    /// * `path` - Relative path like "sources/MyModule.move"
    /// * `content` - Move source code
    pub fn add_source(&mut self, path: String, content: String) {
        self.sources.insert(path, content);
    }

    /// Add a named address mapping
    pub fn add_address(&mut self, name: String, address: String) {
        self.metadata.add_address(name, address);
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, name: String, path: String) {
        self.metadata.add_dependency(name, path);
    }

    /// Build the package and generate bytecode for all modules
    ///
    /// # Returns
    /// CompilationResult with combined bytecode or errors
    pub fn build(&self) -> CompilationResult {
        // TODO: Implement multi-file package compilation
        // For MVP, this is a placeholder
        CompilationResult::new_failure(vec![
            "Multi-file package compilation not yet implemented".to_string(),
            "Use compile_module for single files".to_string(),
        ])
    }

    /// Get package metadata as JSON
    pub fn get_metadata(&self) -> String {
        self.metadata.to_json()
    }

    /// Get list of source files
    pub fn get_sources(&self) -> Vec<String> {
        self.sources.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_creation() {
        let mut pkg = MovePackage::new("TestPackage".to_string(), "0.1.0".to_string());
        pkg.add_source("sources/test.move".to_string(), "module 0x1::Test {}".to_string());
        pkg.add_address("std".to_string(), "0x1".to_string());

        assert_eq!(pkg.get_sources().len(), 1);
    }
}
