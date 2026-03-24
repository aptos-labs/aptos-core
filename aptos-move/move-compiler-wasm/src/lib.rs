// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Move Compiler WebAssembly Bindings
//!
//! This library provides browser and Node.js compatible bindings for the Move compiler.
//!
//! # Features
//!
//! - Compile Move modules and scripts
//! - Build Move packages with dependencies
//! - Lint and verify Move code
//! - Generate bytecode in the browser
//!
//! # Example
//!
//! ```javascript
//! import init, { compile_module } from './pkg/move_compiler_wasm.js';
//!
//! await init();
//!
//! const source = `
//! module 0x42::HelloWorld {
//!     public fun hello(): u64 {
//!         42
//!     }
//! }
//! `;
//!
//! const result = compile_module(source, "0x42", "HelloWorld");
//! if (result.success) {
//!     console.log("Bytecode:", result.bytecode);
//! } else {
//!     console.error("Errors:", result.errors);
//! }
//! ```

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod compiler;
mod package;
mod error;
mod vfs;
mod memfs;
mod stdlib;

pub use compiler::*;
pub use package::*;
pub use error::*;
pub use vfs::*;

/// Initialize panic hook for better error messages in browser console
#[wasm_bindgen]
pub fn init_panic_hook() {
    // Feature disabled due to build issues
    // #[cfg(feature = "console_error_panic_hook")]
    // console_error_panic_hook::set_once();
}

/// Get library version and build information
#[wasm_bindgen]
pub fn get_version_info() -> String {
    serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "build_type": if cfg!(debug_assertions) { "debug" } else { "release" },
        "target": "wasm32-unknown-unknown"
    }).to_string()
}

/// Compilation result returned to JavaScript
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompilationResult {
    success: bool,
    bytecode: Vec<u8>,
    errors: Vec<String>,
    warnings: Vec<String>,
}

#[wasm_bindgen]
impl CompilationResult {
    /// Check if compilation was successful
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    /// Get compiled bytecode (only if successful)
    #[wasm_bindgen(getter)]
    pub fn bytecode(&self) -> Vec<u8> {
        self.bytecode.clone()
    }

    /// Get compilation errors as JSON array
    #[wasm_bindgen(getter)]
    pub fn errors(&self) -> String {
        serde_json::to_string(&self.errors).unwrap_or_default()
    }

    /// Get compilation warnings as JSON array
    #[wasm_bindgen(getter)]
    pub fn warnings(&self) -> String {
        serde_json::to_string(&self.warnings).unwrap_or_default()
    }

    /// Get full result as JSON
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

impl CompilationResult {
    /// Create a successful compilation result
    pub fn new_success(bytecode: Vec<u8>, warnings: Vec<String>) -> Self {
        Self {
            success: true,
            bytecode,
            errors: vec![],
            warnings,
        }
    }

    /// Create a failed compilation result
    pub fn new_failure(errors: Vec<String>) -> Self {
        Self {
            success: false,
            bytecode: vec![],
            errors,
            warnings: vec![],
        }
    }
}

/// Package metadata for Move.toml
#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageMetadata {
    name: String,
    version: String,
    addresses: BTreeMap<String, String>,
    dependencies: BTreeMap<String, String>,
}

#[wasm_bindgen]
impl PackageMetadata {
    /// Create new package metadata
    #[wasm_bindgen(constructor)]
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            addresses: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// Add an address mapping
    pub fn add_address(&mut self, name: String, address: String) {
        self.addresses.insert(name, address);
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, name: String, path: String) {
        self.dependencies.insert(name, path);
    }

    /// Convert to JSON
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = get_version_info();
        assert!(info.contains("move-compiler-wasm"));
    }

    #[test]
    fn test_compilation_result() {
        let result = CompilationResult::new_success(vec![1, 2, 3], vec![]);
        assert!(result.success());
        assert_eq!(result.bytecode(), vec![1, 2, 3]);
    }
}
