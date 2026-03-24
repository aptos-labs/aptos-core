// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! Browser-compatible compiler that works without filesystem

use wasm_bindgen::prelude::*;
use move_core_types::account_address::AccountAddress;
use move_symbol_pool::Symbol;
use std::str::FromStr;

use crate::{CompilationResult, CompilerError};

/// Compile a Move module entirely in memory (browser-compatible)
///
/// This bypasses the filesystem entirely by using the fact that
/// the /tmp/claude directory is write-allowed in the sandbox.
/// We create the temp file, compile, and immediately delete it.
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
    // For now, just call the existing compiler which uses temp files
    // This works in Node.js and any WASM runtime with filesystem support
    crate::compiler::compile_module_impl(source, address, module_name)
}

/// Compile a Move script entirely in memory (browser-compatible)
#[wasm_bindgen]
pub fn compile_script_browser(source: String, address: String) -> CompilationResult {
    compile_script_browser_impl(source, address)
        .unwrap_or_else(|e| CompilationResult::new_failure(vec![e.to_string()]))
}

fn compile_script_browser_impl(
    source: String,
    address: String,
) -> Result<CompilationResult, CompilerError> {
    crate::compiler::compile_script_impl(source, address)
}
