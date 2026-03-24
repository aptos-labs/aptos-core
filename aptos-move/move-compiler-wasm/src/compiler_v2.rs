// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Browser-compatible compiler using in-memory sources

use wasm_bindgen::prelude::*;
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol;
use std::str::FromStr;
use std::collections::BTreeMap;

use crate::{CompilationResult, CompilerError, VirtualFS};

/// Compile a Move module using in-memory sources (browser-compatible)
///
/// This version doesn't use std::fs at all - everything stays in memory
#[wasm_bindgen]
pub fn compile_module_browser(source: String, address: String, module_name: String) -> CompilationResult {
    compile_module_browser_impl(source, address, module_name)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_module_browser_impl(
    source: String,
    address: String,
    module_name: String,
) -> Result<CompilationResult, CompilerError> {
    // Parse the address
    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    // Extract named address from source
    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    // Create virtual filesystem
    let vfs = VirtualFS::new();
    let virtual_path = format!("{}.move", module_name);
    vfs.add_text_file(&virtual_path, &source);

    // Use legacy compiler API which accepts source strings directly
    compile_with_legacy_api(&source, addr, named_addr, &module_name)
}

/// Use the legacy move-compiler API that accepts sources as strings
fn compile_with_legacy_api(
    source: &str,
    address: AccountAddress,
    named_addr: Symbol,
    module_name: &str,
) -> Result<CompilationResult, CompilerError> {
    use legacy_move_compiler::{
        compiled_unit::CompiledUnit,
        Compiler,
        Flags,
    };

    // Create named address mapping
    let mut named_addresses = BTreeMap::new();
    named_addresses.insert(named_addr, address);

    // Compile using legacy compiler (which accepts strings directly)
    let file_name = format!("{}.move", module_name);
    let sources = vec![(file_name, source.to_string())];

    let (files, units_result) = Compiler::from_sources(
        sources,
        vec![], // No dependencies for now
        named_addresses,
        Flags::empty(),
    )
    .build()
    .map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|e| format!("{}", e))
                .collect()
        )
    })?;

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

/// Compile a Move script using in-memory sources (browser-compatible)
#[wasm_bindgen]
pub fn compile_script_browser(source: String, address: String) -> CompilationResult {
    compile_script_browser_impl(source, address)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_script_browser_impl(
    source: String,
    address: String,
) -> Result<CompilationResult, CompilerError> {
    use legacy_move_compiler::{
        compiled_unit::CompiledUnit,
        Compiler,
        Flags,
    };

    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    let mut named_addresses = BTreeMap::new();
    named_addresses.insert(named_addr, addr);

    let sources = vec![("script.move".to_string(), source)];

    let (files, units_result) = Compiler::from_sources(
        sources,
        vec![],
        named_addresses,
        Flags::empty(),
    )
    .build()
    .map_err(|errors| {
        CompilerError::CompilationFailed(
            errors.into_iter()
                .map(|e| format!("{}", e))
                .collect()
        )
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_simple_module() {
        let source = r#"
module 0x1::Test {
    public fun hello(): u64 {
        42
    }
}
        "#;

        let result = compile_module_browser_impl(
            source.to_string(),
            "0x1".to_string(),
            "Test".to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert!(!result.bytecode.is_empty());
    }

    #[test]
    fn test_compile_with_error() {
        let source = r#"
module 0x1::Broken {
    public fun broken() {
        this_does_not_exist();
    }
}
        "#;

        let result = compile_module_browser_impl(
            source.to_string(),
            "0x1".to_string(),
            "Broken".to_string(),
        );

        // Should return error result, not panic
        assert!(result.is_err() || !result.unwrap().success);
    }
}
