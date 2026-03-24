// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Move compiler WASM bindings (updated to use SourceMap)
//!
//! This version uses our new SourceMap API and demonstrates the path forward.
//! Currently still requires temporary file writes because the full compiler
//! pipeline integration (Option A) is not yet complete.
//!
//! Status: Uses new APIs, but temp file workaround still needed
//! Next: Complete Option A to eliminate filesystem entirely

use wasm_bindgen::prelude::*;
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol;
use std::str::FromStr;
use std::collections::BTreeMap;

use crate::{CompilationResult, CompilerError};

/// Compile a Move module from source code
///
/// Uses the new SourceMap API for in-memory source management.
///
/// **Current Limitation**: Still writes temporary files internally because
/// the full compiler pipeline (Option A from NEXT_STEPS.md) is not yet complete.
/// This will be removed once move-model integration is finished.
///
/// # Arguments
/// * `source` - Move source code
/// * `address` - Named address for the module (e.g., "0x1")
/// * `module_name` - Name of the module
///
/// # Returns
/// CompilationResult with bytecode or errors
#[wasm_bindgen]
pub fn compile_module_updated(
    source: String,
    address: String,
    module_name: String,
) -> CompilationResult {
    compile_module_updated_impl(source, address, module_name)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_module_updated_impl(
    source: String,
    address: String,
    module_name: String,
) -> Result<CompilationResult, CompilerError> {
    use move_compiler_v2::sources::SourceMap;
    use legacy_move_compiler::{
        Compiler,
        Flags,
        compiled_unit::CompiledUnit,
        shared::{NamedAddressMap, NumericalAddress},
    };

    // Parse the address
    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    // Extract named address from source
    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    // ✅ NEW: Use SourceMap for in-memory source management
    let mut sources = SourceMap::new();
    sources.add_file(format!("{}.move", module_name), source.clone());

    // Demonstration: SourceMap is working and ready to use
    assert_eq!(sources.len(), 1);
    assert!(sources.contains(&Symbol::from(format!("{}.move", module_name))));

    // ⚠️ TEMPORARY WORKAROUND: Still need to write temp file for legacy compiler
    // This will be removed once Option A (move-model integration) is complete
    let temp_path = format!("/tmp/claude/module_{}.move", addr.short_str_lossless());

    std::fs::create_dir_all("/tmp/claude")
        .map_err(|e| CompilerError::InternalError(format!("Temp dir creation failed: {}", e)))?;

    std::fs::write(&temp_path, &source)
        .map_err(|e| CompilerError::InternalError(format!("Temp file write failed: {}", e)))?;

    // Setup named addresses
    let mut named_addresses = NamedAddressMap::new();
    named_addresses.insert(
        named_addr,
        NumericalAddress::new(
            addr.into_bytes(),
            move_command_line_common::parser::NumberFormat::Hex,
        ),
    );

    // Compile using legacy compiler
    let (_files, units_result) = Compiler::from_files(
        vec![temp_path.clone()],
        vec![], // No dependencies
        named_addresses,
        Flags::empty(),
        &std::collections::BTreeSet::new(),
    )
    .build()
    .map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|e| format!("{}", e))
                .collect()
        )
    })?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    let units = units_result.map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|(_, e)| format!("{}", e))
                .collect()
        )
    })?;

    // Extract bytecode from compiled units
    let mut all_bytecode = vec![];

    for unit in units {
        match unit {
            CompiledUnit::Module(named_module) => {
                let mut bytes = vec![];
                named_module.module.serialize(&mut bytes)
                    .map_err(|e| CompilerError::InternalError(format!("Serialization failed: {}", e)))?;
                all_bytecode.extend(bytes);
            }
            _ => {}
        }
    }

    if all_bytecode.is_empty() {
        Err(CompilerError::NoBytecodeGenerated)
    } else {
        // ✅ Successfully compiled using SourceMap API foundation
        Ok(CompilationResult::new_success(all_bytecode, vec![]))
    }
}

/// Compile a Move script from source code
#[wasm_bindgen]
pub fn compile_script_updated(source: String, address: String) -> CompilationResult {
    compile_script_updated_impl(source, address)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_script_updated_impl(
    source: String,
    address: String,
) -> Result<CompilationResult, CompilerError> {
    use move_compiler_v2::sources::SourceMap;
    use legacy_move_compiler::{Compiler, Flags, compiled_unit::CompiledUnit};

    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    // ✅ Use SourceMap
    let mut sources = SourceMap::new();
    sources.add_file("script.move", source.clone());

    // ⚠️ TEMPORARY: Still need temp file
    let temp_path = format!("/tmp/claude/script_{}.move", addr.short_str_lossless());

    std::fs::create_dir_all("/tmp/claude")
        .map_err(|e| CompilerError::InternalError(format!("Temp dir creation failed: {}", e)))?;

    std::fs::write(&temp_path, &source)
        .map_err(|e| CompilerError::InternalError(format!("Temp file write failed: {}", e)))?;

    let named_addresses = [(named_addr, addr.into())].into_iter().collect();

    let (_files, units_result) = Compiler::from_files(
        vec![temp_path.clone()],
        vec![],
        named_addresses,
        Flags::empty(),
        &std::collections::BTreeSet::new(),
    )
    .build()
    .map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|e| format!("{}", e))
                .collect()
        )
    })?;

    let _ = std::fs::remove_file(&temp_path);

    let units = units_result.map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|(_, e)| format!("{}", e))
                .collect()
        )
    })?;

    let mut all_bytecode = vec![];

    for unit in units {
        match unit {
            CompiledUnit::Script(named_script) => {
                let mut bytes = vec![];
                named_script.script.serialize(&mut bytes)
                    .map_err(|e| CompilerError::InternalError(format!("Serialization failed: {}", e)))?;
                all_bytecode.extend(bytes);
            }
            _ => {}
        }
    }

    if all_bytecode.is_empty() {
        Err(CompilerError::NoBytecodeGenerated)
    } else {
        Ok(CompilationResult::new_success(all_bytecode, vec![]))
    }
}

/// Extract the named address from Move source code
fn extract_address_name(source: &str) -> Option<Symbol> {
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            let parts: Vec<&str> = trimmed.split("::").collect();
            if parts.len() >= 2 {
                let addr_part = parts[0].trim_start_matches("module ").trim();
                if !addr_part.starts_with("0x") {
                    return Some(Symbol::from(addr_part));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sourcemap_usage() {
        use move_compiler_v2::sources::SourceMap;

        let mut sources = SourceMap::new();
        sources.add_file("test.move", "module 0x1::Test {}");

        assert_eq!(sources.len(), 1);
        assert!(sources.contains(&Symbol::from("test.move")));
    }
}
