// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Move compiler WASM bindings (filesystem-free implementation)
//!
//! This module provides true browser-native compilation using the new
//! filesystem-free API (`run_move_compiler_from_sources`).
//!
//! **NO FILESYSTEM ACCESS** - compiles directly from in-memory strings.

use wasm_bindgen::prelude::*;
use move_compiler_v2::{run_move_compiler_from_sources, sources::SourceMap, Options};
use move_compiler_v2::diagnostics::Emitter;
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol;
use std::str::FromStr;
use codespan::{FileId, Files};
use codespan_reporting::diagnostic::Diagnostic;

use crate::{CompilationResult, CompilerError};
use crate::stdlib;

/// Simple string-based error emitter for WASM
struct StringEmitter {
    errors: Vec<String>,
}

impl StringEmitter {
    fn new() -> Self {
        Self { errors: vec![] }
    }
}

impl Emitter for StringEmitter {
    fn emit(&mut self, _source_files: &Files<String>, diag: &Diagnostic<FileId>) {
        // Simple error collection - just grab the message
        self.errors.push(format!("{:?}: {}", diag.severity, diag.message));
    }
}

/// Compile a single Move module from source code (filesystem-free!)
///
/// # Arguments
/// * `source` - Move source code
/// * `address` - Named address for the module (e.g., "0x1")
/// * `module_name` - Name of the module
///
/// # Returns
/// CompilationResult with bytecode or errors
///
/// # Implementation
/// Uses the new `run_move_compiler_from_sources()` API - no temp files needed!
#[wasm_bindgen]
pub fn compile_module(source: String, address: String, module_name: String) -> CompilationResult {
    compile_module_impl(source, address, module_name)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_module_impl(
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

    // ✅ NEW: Use SourceMap for in-memory source management
    let mut sources = SourceMap::new();
    sources.add_file(format!("{}.move", module_name), source);

    // Build named address mappings: well-known addresses + user's module address
    let mut address_map = stdlib::well_known_addresses();
    address_map.push((named_addr.to_string(), addr));

    // Load bundled move-stdlib as dependencies
    let deps = stdlib::stdlib_source_map();

    let mut emitter = StringEmitter::new();
    let options = Options::default();

    let result = run_move_compiler_from_sources(
        &mut emitter,
        sources,
        deps,
        address_map,
        options,
    );

    match result {
        Ok((_env, units)) => {
            let mut all_bytecode = vec![];

            for unit in units {
                match unit {
                    legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) => {
                        let mut bytes = vec![];
                        module.named_module.module.serialize(&mut bytes)
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
        Err(e) => {
            if !emitter.errors.is_empty() {
                Err(CompilerError::CompilationFailed(emitter.errors))
            } else {
                Err(CompilerError::InternalError(format!("Compilation failed: {}", e)))
            }
        }
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

/// Compile a single Move module with additional dependency sources.
///
/// `deps_json` is a JSON array of `{"path":"...", "content":"..."}` objects.
/// These are treated as library sources (dependencies) so the compiler can
/// resolve `use` imports that are not part of the bundled move-stdlib.
///
/// `extra_named_addresses_json` is a JSON object like `{"name":"0x1",...}`.
#[wasm_bindgen]
pub fn compile_module_with_deps(
    source: String,
    address: String,
    module_name: String,
    deps_json: String,
    extra_named_addresses_json: String,
) -> CompilationResult {
    compile_module_with_deps_impl(source, address, module_name, deps_json, extra_named_addresses_json)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_module_with_deps_impl(
    source: String,
    address: String,
    module_name: String,
    deps_json: String,
    extra_named_addresses_json: String,
) -> Result<CompilationResult, CompilerError> {
    use serde::Deserialize;
    use std::collections::BTreeMap;

    #[derive(Deserialize)]
    struct DepFile {
        path: String,
        content: String,
    }

    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    let mut sources = SourceMap::new();
    sources.add_file(format!("{}.move", module_name), source);

    // Start from the bundled stdlib deps
    let mut deps = stdlib::stdlib_source_map();

    // Add user-provided dependency sources
    if !deps_json.is_empty() && deps_json != "[]" {
        let dep_files: Vec<DepFile> = serde_json::from_str(&deps_json)
            .unwrap_or_default();
        for dep in dep_files {
            deps.add_file(dep.path, dep.content);
        }
    }

    // Build named address mappings
    let mut address_map = stdlib::well_known_addresses();
    address_map.push((named_addr.to_string(), addr));

    // Add extra named addresses
    if !extra_named_addresses_json.is_empty() && extra_named_addresses_json != "{}" {
        let extra: BTreeMap<String, String> = serde_json::from_str(&extra_named_addresses_json)
            .unwrap_or_default();
        for (name, addr_str) in extra {
            if let Ok(a) = AccountAddress::from_hex_literal(&addr_str)
                .or_else(|_| AccountAddress::from_str(&addr_str))
            {
                address_map.push((name, a));
            }
        }
    }

    let mut emitter = StringEmitter::new();
    let options = Options::default();

    let result = run_move_compiler_from_sources(
        &mut emitter,
        sources,
        deps,
        address_map,
        options,
    );

    match result {
        Ok((_env, units)) => {
            let mut all_bytecode = vec![];
            for unit in units {
                match unit {
                    legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Module(module) => {
                        let mut bytes = vec![];
                        module.named_module.module.serialize(&mut bytes)
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
        Err(e) => {
            if !emitter.errors.is_empty() {
                Err(CompilerError::CompilationFailed(emitter.errors))
            } else {
                Err(CompilerError::InternalError(format!("Compilation failed: {}", e)))
            }
        }
    }
}

/// Compile a Move script from source code (filesystem-free!)
#[wasm_bindgen]
pub fn compile_script(source: String, address: String) -> CompilationResult {
    compile_script_impl(source, address)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_script_impl(
    source: String,
    address: String,
) -> Result<CompilationResult, CompilerError> {
    let addr = AccountAddress::from_hex_literal(&address)
        .or_else(|_| AccountAddress::from_str(&address))
        .map_err(|e| CompilerError::InvalidAddress(format!("Invalid address '{}': {}", address, e)))?;

    let named_addr = extract_address_name(&source)
        .unwrap_or_else(|| Symbol::from("default_addr"));

    let mut sources = SourceMap::new();
    sources.add_file("script.move", source);

    // Build named address mappings: well-known addresses + user's address
    let mut address_map = stdlib::well_known_addresses();
    address_map.push((named_addr.to_string(), addr));

    // Load bundled move-stdlib as dependencies
    let deps = stdlib::stdlib_source_map();

    let mut emitter = StringEmitter::new();
    let options = Options::default();

    let result = run_move_compiler_from_sources(
        &mut emitter,
        sources,
        deps,
        address_map,
        options,
    );

    match result {
        Ok((_env, units)) => {
            let mut all_bytecode = vec![];

            for unit in units {
                match unit {
                    legacy_move_compiler::compiled_unit::AnnotatedCompiledUnit::Script(script) => {
                        let mut bytes = vec![];
                        script.named_script.script.serialize(&mut bytes)
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
        Err(e) => {
            if !emitter.errors.is_empty() {
                Err(CompilerError::CompilationFailed(emitter.errors))
            } else {
                Err(CompilerError::InternalError(format!("Compilation failed: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_address_name() {
        let source = r#"
        module my_addr::Test {
            public fun hello() {}
        }
        "#;
        assert_eq!(extract_address_name(source), Some(Symbol::from("my_addr")));
    }

    #[test]
    fn test_compile_simple_module() {
        let source = r#"
        module 0x1::Test {
            public fun answer(): u64 {
                42
            }
        }
        "#;

        let result = compile_module(source.to_string(), "0x1".to_string(), "Test".to_string());
        if !result.success() {
            eprintln!("Errors: {}", result.errors());
        }
        assert!(result.success(), "Compilation should succeed");
        assert!(!result.bytecode().is_empty(), "Should generate bytecode");
    }

    #[test]
    fn test_compile_module_with_stdlib_imports() {
        let source = r#"
        module 0x42::Counter {
            use std::signer;

            struct CounterResource has key {
                value: u64
            }

            public fun init(account: &signer) {
                move_to(account, CounterResource { value: 0 });
            }

            public fun get_value(addr: address): u64 acquires CounterResource {
                borrow_global<CounterResource>(addr).value
            }

            public fun increment(account: &signer) acquires CounterResource {
                let counter = borrow_global_mut<CounterResource>(signer::address_of(account));
                counter.value = counter.value + 1;
            }
        }
        "#;

        let result = compile_module(source.to_string(), "0x42".to_string(), "Counter".to_string());
        if !result.success() {
            eprintln!("Errors: {}", result.errors());
        }
        assert!(result.success(), "Module with std::signer import should compile");
        assert!(!result.bytecode().is_empty(), "Should generate bytecode");
    }
}
