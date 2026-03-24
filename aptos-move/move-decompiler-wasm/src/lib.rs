// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Move Decompiler for WebAssembly
//!
//! This library provides WASM bindings for Move bytecode analysis:
//! - Disassemble bytecode to human-readable assembly
//! - Decompile bytecode back to Move source code
//! - Extract metadata and version information
//!
//! # Example (JavaScript)
//! ```javascript
//! import init, { decompile_module, disassemble_module } from './move_decompiler_wasm.js';
//!
//! await init();
//!
//! // Decompile bytecode to Move source
//! const bytecode = new Uint8Array([...]); // Your .mv file
//! const source = decompile_module(bytecode);
//! console.log(source);
//!
//! // Or disassemble to assembly
//! const assembly = disassemble_module(bytecode);
//! console.log(assembly);
//! ```

use move_binary_format::{
    access::ModuleAccess,
    file_format::{CompiledModule, CompiledScript},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Result type for operations that can fail
type Result<T> = std::result::Result<T, JsValue>;

/// Convert Rust errors to JavaScript exceptions
fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Metadata extracted from Move bytecode
#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct BytecodeMetadata {
    name: String,
    address: Option<String>,
    version: u32,
    function_count: usize,
    struct_count: usize,
    dependencies: Vec<String>,
}

#[wasm_bindgen]
impl BytecodeMetadata {
    /// Get module name
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get module address
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> Option<String> {
        self.address.clone()
    }

    /// Get bytecode version
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Get function count
    #[wasm_bindgen(getter, js_name = functionCount)]
    pub fn function_count(&self) -> usize {
        self.function_count
    }

    /// Get struct count
    #[wasm_bindgen(getter, js_name = structCount)]
    pub fn struct_count(&self) -> usize {
        self.struct_count
    }

    /// Get dependencies as JSON array string
    #[wasm_bindgen(js_name = getDependencies)]
    pub fn dependencies(&self) -> String {
        serde_json::to_string(&self.dependencies).unwrap_or_else(|_| "[]".to_string())
    }

    /// Convert to JSON string
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(to_js_error)
    }
}

/// Decompile Move module bytecode to Move source code
///
/// Takes raw bytecode bytes and returns decompiled Move source code.
///
/// # Arguments
/// * `bytecode` - Raw bytes from a .mv file (compiled Move module)
///
/// # Returns
/// Decompiled Move source code as a string
///
/// # Errors
/// Returns error if:
/// - Bytecode is invalid or corrupted
/// - Bytecode format version is unsupported
/// - Decompilation fails
///
/// # Example
/// ```javascript
/// const source = decompile_module(bytecodeBytes);
/// console.log(source);
/// // module 0x1::MyModule {
/// //     public fun hello(): u64 { 42 }
/// // }
/// ```
#[wasm_bindgen]
pub fn decompile_module(bytecode: &[u8]) -> Result<String> {
    use move_decompiler::{Decompiler, Options as DecompilerOptions};

    // Deserialize bytecode
    let module = CompiledModule::deserialize(bytecode).map_err(to_js_error)?;

    // Create decompiler with default options
    let mut decompiler = Decompiler::new(DecompilerOptions::default());

    // Create empty source map (since we don't have source maps from bytecode alone)
    let source_map = decompiler.empty_source_map("module.mv", bytecode);

    // Decompile to Move source
    decompiler
        .decompile_module(module, source_map)
        .map_err(to_js_error)
}

/// Decompile Move script bytecode to Move source code
///
/// # Arguments
/// * `bytecode` - Raw bytes from a compiled Move script
///
/// # Returns
/// Decompiled Move script source code
#[wasm_bindgen]
pub fn decompile_script(bytecode: &[u8]) -> Result<String> {
    use move_decompiler::{Decompiler, Options as DecompilerOptions};

    let script = CompiledScript::deserialize(bytecode).map_err(to_js_error)?;

    // Create decompiler with default options
    let mut decompiler = Decompiler::new(DecompilerOptions::default());

    // Create empty source map
    let source_map = decompiler.empty_source_map("script.mv", bytecode);

    // Decompile to Move source
    decompiler
        .decompile_script(script, source_map)
        .map_err(to_js_error)
}

/// Disassemble Move module bytecode to human-readable assembly
///
/// Produces a detailed assembly-like representation of the bytecode,
/// showing all instructions, function signatures, and struct layouts.
///
/// # Arguments
/// * `bytecode` - Raw bytes from a .mv file
///
/// # Returns
/// Disassembled bytecode as formatted text
///
/// # Example
/// ```javascript
/// const assembly = disassemble_module(bytecodeBytes);
/// console.log(assembly);
/// // // Module: 0x1::MyModule
/// // // Function: hello
/// // public hello(): u64 {
/// //   L0: LdU64(42)
/// //   L1: Ret
/// // }
/// ```
#[wasm_bindgen]
pub fn disassemble_module(bytecode: &[u8]) -> Result<String> {
    let module = CompiledModule::deserialize(bytecode).map_err(to_js_error)?;
    move_asm::disassembler::disassemble_module(String::new(), &module).map_err(to_js_error)
}

/// Disassemble Move script bytecode
///
/// # Arguments
/// * `bytecode` - Raw bytes from a compiled Move script
///
/// # Returns
/// Disassembled script bytecode
#[wasm_bindgen]
pub fn disassemble_script(bytecode: &[u8]) -> Result<String> {
    let script = CompiledScript::deserialize(bytecode).map_err(to_js_error)?;
    move_asm::disassembler::disassemble_script(String::new(), &script).map_err(to_js_error)
}

/// Extract metadata from Move module bytecode
///
/// Returns structured information about the module without full decompilation.
///
/// # Arguments
/// * `bytecode` - Raw bytes from a .mv file
///
/// # Returns
/// BytecodeMetadata object with module information
///
/// # Example
/// ```javascript
/// const metadata = get_module_metadata(bytecodeBytes);
/// console.log(metadata.name); // "MyModule"
/// console.log(metadata.to_json()); // Full JSON representation
/// ```
#[wasm_bindgen]
pub fn get_module_metadata(bytecode: &[u8]) -> Result<BytecodeMetadata> {
    let module = CompiledModule::deserialize(bytecode).map_err(to_js_error)?;

    let name = module.self_id().name().to_string();
    let address = Some(module.self_id().address().to_string());
    let version = module.version();
    let function_count = module.function_defs.len();
    let struct_count = module.struct_defs.len();

    let dependencies: Vec<String> = module
        .module_handles
        .iter()
        .filter_map(|handle| {
            let addr = module.address_identifier_at(handle.address);
            let name = module.identifier_at(handle.name);
            let module_id = format!("{}::{}", addr, name);
            // Skip self-reference
            if module_id != module.self_id().to_string() {
                Some(module_id)
            } else {
                None
            }
        })
        .collect();

    Ok(BytecodeMetadata {
        name,
        address,
        version,
        function_count,
        struct_count,
        dependencies,
    })
}

/// Verify Move module bytecode integrity
///
/// Performs basic bytecode verification without full VM validation.
///
/// # Arguments
/// * `bytecode` - Raw bytes to verify
///
/// # Returns
/// - `Ok(true)` if bytecode is valid
/// - `Err(...)` if bytecode is invalid
///
/// # Example
/// ```javascript
/// try {
///     verify_module(bytecodeBytes);
///     console.log("✓ Bytecode is valid");
/// } catch (e) {
///     console.error("✗ Invalid bytecode:", e);
/// }
/// ```
#[wasm_bindgen]
pub fn verify_module(bytecode: &[u8]) -> Result<bool> {
    // Attempt to deserialize - this validates structure
    CompiledModule::deserialize(bytecode).map_err(to_js_error)?;
    Ok(true)
}

/// Verify Move script bytecode integrity
#[wasm_bindgen]
pub fn verify_script(bytecode: &[u8]) -> Result<bool> {
    CompiledScript::deserialize(bytecode).map_err(to_js_error)?;
    Ok(true)
}

/// Get library version information
///
/// Returns version and build information for the WASM library.
///
/// # Returns
/// JSON string with version details
#[wasm_bindgen]
pub fn get_version_info() -> String {
    serde_json::json!({
        "name": "move-decompiler-wasm",
        "version": env!("CARGO_PKG_VERSION"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "features": [
            "decompile_module",
            "decompile_script",
            "disassemble_module",
            "disassemble_script",
            "get_module_metadata",
            "verify_module",
            "verify_script"
        ],
        "build": {
            "target": "wasm32-unknown-unknown",
            "profile": if cfg!(debug_assertions) { "debug" } else { "release" }
        }
    })
    .to_string()
}

/// Initialize panic hook for better error messages in browser console
///
/// Call this once when your WASM module loads to get better error messages.
///
/// # Example
/// ```javascript
/// import init, { init_panic_hook } from './move_decompiler_wasm.js';
///
/// await init();
/// init_panic_hook(); // Better error messages in browser
/// ```
#[wasm_bindgen]
pub fn init_panic_hook() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = get_version_info();
        assert!(info.contains("move-decompiler-wasm"));
        assert!(info.contains("version"));
    }
}
