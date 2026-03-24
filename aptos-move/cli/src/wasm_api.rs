// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! WASM API for aptos-move-cli
//!
//! This module provides a minimal WASM-compatible API for core Move operations.
//! It's designed for evaluation purposes to test WASM feasibility.
//!
//! **Phase 3 MVP:** Disassemble and Decompile commands (no filesystem dependencies)

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use move_binary_format::CompiledModule;

#[cfg(target_arch = "wasm32")]
use move_decompiler::{Decompiler, Options as DecompilerOptions};

/// Disassemble Move bytecode into textual assembly representation.
///
/// # Arguments
/// * `bytecode` - Raw Move bytecode bytes (.mv file contents)
/// * `is_script` - Whether the bytecode is a script (true) or module (false)
///
/// # Returns
/// * `Ok(String)` - Disassembled assembly text
/// * `Err(JsValue)` - Error message if disassembly fails
///
/// # Example (JavaScript)
/// ```javascript
/// import init, { disassemble_bytecode } from './aptos_move_cli.js';
///
/// await init();
/// const bytecode = new Uint8Array([...]); // Load .mv file
/// const assembly = disassemble_bytecode(bytecode, false);
/// console.log(assembly);
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn disassemble_bytecode(bytecode: Vec<u8>, is_script: bool) -> Result<String, JsValue> {
    // Parse bytecode
    let compiled_module = CompiledModule::deserialize(&bytecode)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize bytecode: {}", e)))?;

    // Generate disassembly
    let disassembly = if is_script {
        // For scripts, we'd need CompiledScript, but for MVP we'll handle modules only
        return Err(JsValue::from_str(
            "Script disassembly not yet implemented in WASM API",
        ));
    } else {
        // For modules, use the binary format's display implementation
        format!("{:#?}", compiled_module)
    };

    Ok(disassembly)
}

/// Decompile Move bytecode into Move source code.
///
/// # Arguments
/// * `bytecode` - Raw Move bytecode bytes (.mv file contents)
/// * `is_script` - Whether the bytecode is a script (true) or module (false)
///
/// # Returns
/// * `Ok(String)` - Decompiled Move source code
/// * `Err(JsValue)` - Error message if decompilation fails
///
/// # Example (JavaScript)
/// ```javascript
/// import init, { decompile_bytecode } from './aptos_move_cli.js';
///
/// await init();
/// const bytecode = new Uint8Array([...]); // Load .mv file
/// const source = decompile_bytecode(bytecode, false);
/// console.log(source);
/// ```
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decompile_bytecode(bytecode: Vec<u8>, is_script: bool) -> Result<String, JsValue> {
    // Parse bytecode
    let compiled_module = CompiledModule::deserialize(&bytecode)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize bytecode: {}", e)))?;

    // Decompile using move-decompiler
    if is_script {
        return Err(JsValue::from_str(
            "Script decompilation not yet implemented in WASM API",
        ));
    }

    let options = DecompilerOptions::default();
    let decompiler = Decompiler::new(compiled_module, options);

    let source_text = decompiler
        .source_text()
        .map_err(|e| JsValue::from_str(&format!("Decompilation failed: {}", e)))?;

    Ok(source_text)
}

/// Get version information about the WASM module.
///
/// Returns a JSON string with build information.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_version_info() -> String {
    serde_json::json!({
        "crate": "aptos-move-cli",
        "version": env!("CARGO_PKG_VERSION"),
        "wasm_api_version": "0.1.0-eval",
        "phase": "3-proof-of-concept",
        "supported_commands": [
            "disassemble_bytecode",
            "decompile_bytecode"
        ]
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let info = get_version_info();
        assert!(info.contains("aptos-move-cli"));
    }
}
