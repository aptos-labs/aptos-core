// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Move Decompiler and Disassembler for WebAssembly
//!
//! This library provides WASM bindings for Move bytecode analysis. It is
//! intentionally scoped to **read-only** bytecode tooling and does *not* embed a
//! Move compiler:
//! - Disassemble bytecode to human-readable assembly
//! - Decompile bytecode back to Move source code
//! - Extract metadata and version information
//!
//! It is built against the latest Aptos bytecode format and supports bytecode
//! version 10 (the current maximum/default).
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
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// Result type for the WASM-facing API.
type Result<T> = std::result::Result<T, JsValue>;

/// Convert Rust errors to JavaScript exceptions.
///
/// NOTE: This must only ever run on the `wasm32` target. On non-wasm hosts,
/// `JsValue` construction is unimplemented and aborts the process. That is why
/// all real logic lives in the target-independent `*_impl` helpers below (which
/// return `anyhow::Result` and are unit-tested on the host), while the
/// `#[wasm_bindgen]` entry points are thin wrappers that only convert the error.
fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Deserialize a [`CompiledModule`] from raw bytecode.
fn deserialize_module(bytecode: &[u8]) -> anyhow::Result<CompiledModule> {
    CompiledModule::deserialize(bytecode)
        .map_err(|e| anyhow::anyhow!("failed to deserialize module bytecode: {e:?}"))
}

/// Deserialize a [`CompiledScript`] from raw bytecode.
fn deserialize_script(bytecode: &[u8]) -> anyhow::Result<CompiledScript> {
    CompiledScript::deserialize(bytecode)
        .map_err(|e| anyhow::anyhow!("failed to deserialize script bytecode: {e:?}"))
}

/// Core (target-independent) implementation of [`decompile_module`].
fn decompile_module_impl(bytecode: &[u8]) -> anyhow::Result<String> {
    use move_decompiler::{Decompiler, Options as DecompilerOptions};

    let module = deserialize_module(bytecode)?;
    let mut decompiler = Decompiler::new(DecompilerOptions::default());
    // We only have bytecode, so build an empty source map for error reporting.
    let source_map = decompiler.empty_source_map("module.mv", bytecode);
    decompiler.decompile_module(module, source_map)
}

/// Core (target-independent) implementation of [`decompile_script`].
fn decompile_script_impl(bytecode: &[u8]) -> anyhow::Result<String> {
    use move_decompiler::{Decompiler, Options as DecompilerOptions};

    let script = deserialize_script(bytecode)?;
    let mut decompiler = Decompiler::new(DecompilerOptions::default());
    let source_map = decompiler.empty_source_map("script.mv", bytecode);
    decompiler.decompile_script(script, source_map)
}

/// Core (target-independent) implementation of [`disassemble_module`].
fn disassemble_module_impl(bytecode: &[u8]) -> anyhow::Result<String> {
    let module = deserialize_module(bytecode)?;
    move_asm::disassembler::disassemble_module(String::new(), &module)
}

/// Core (target-independent) implementation of [`disassemble_script`].
fn disassemble_script_impl(bytecode: &[u8]) -> anyhow::Result<String> {
    let script = deserialize_script(bytecode)?;
    move_asm::disassembler::disassemble_script(String::new(), &script)
}

/// Core (target-independent) implementation of [`get_module_metadata`].
fn module_metadata_impl(bytecode: &[u8]) -> anyhow::Result<BytecodeMetadata> {
    let module = deserialize_module(bytecode)?;

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
    decompile_module_impl(bytecode).map_err(to_js_error)
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
    decompile_script_impl(bytecode).map_err(to_js_error)
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
    disassemble_module_impl(bytecode).map_err(to_js_error)
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
    disassemble_script_impl(bytecode).map_err(to_js_error)
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
    module_metadata_impl(bytecode).map_err(to_js_error)
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
    deserialize_module(bytecode).map_err(to_js_error)?;
    Ok(true)
}

/// Verify Move script bytecode integrity
#[wasm_bindgen]
pub fn verify_script(bytecode: &[u8]) -> Result<bool> {
    deserialize_script(bytecode).map_err(to_js_error)?;
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
            "compile_module",
            "compile_script",
            "decompile_module",
            "decompile_script",
            "disassemble_module",
            "disassemble_script",
            "get_module_metadata",
            "verify_module",
            "verify_script"
        ],
        "max_bytecode_version": move_binary_format::file_format_common::VERSION_MAX,
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

/// A single compiled unit produced from Move source: its serialized bytecode
/// and whether it is a script (as opposed to a module).
struct CompiledArtifact {
    is_script: bool,
    bytecode: Vec<u8>,
}

/// Core (target-independent) implementation for compiling Move source to
/// bytecode.
///
/// All source is read from memory (the `source`/`deps` arguments), so no
/// filesystem access is performed. This is what allows the Move compiler
/// front-end to run on the `wasm32-unknown-unknown` target. `deps` maps
/// dependency file names to their source text; `named_addresses` maps named
/// address aliases (e.g. `std`) to hex addresses (e.g. `0x1`).
fn compile_impl(
    file_name: &str,
    source: &str,
    named_addresses: &BTreeMap<String, String>,
    deps: &BTreeMap<String, String>,
    bytecode_version: u32,
) -> anyhow::Result<Vec<CompiledArtifact>> {
    use codespan_reporting::term::termcolor::Buffer;
    use legacy_move_compiler::compiled_unit::CompiledUnitEnum;
    use move_compiler_v2::{options::Options, run_move_compiler};

    // Build the in-memory source store (targets + source dependencies).
    let mut sources_content: BTreeMap<String, String> = BTreeMap::new();
    sources_content.insert(file_name.to_string(), source.to_string());
    let mut sources_deps = Vec::new();
    for (dep_name, dep_source) in deps {
        sources_content.insert(dep_name.clone(), dep_source.clone());
        sources_deps.push(dep_name.clone());
    }

    let named_address_mapping = named_addresses
        .iter()
        .map(|(name, addr)| format!("{name}={addr}"))
        .collect::<Vec<_>>();

    let options = Options {
        sources: vec![file_name.to_string()],
        sources_deps,
        dependencies: vec![],
        named_address_mapping,
        sources_content,
        skip_attribute_checks: true,
        print_errors: false,
        ..Options::default()
    };

    // Capture diagnostics into a buffer instead of writing to stderr.
    let mut buffer = Buffer::no_color();
    let result = {
        let mut emitter = options.error_emitter(&mut buffer);
        run_move_compiler(emitter.as_mut(), options.clone())
    };
    let (_env, units) = match result {
        Ok(ok) => ok,
        Err(err) => {
            let diags = String::from_utf8_lossy(&buffer.into_inner()).into_owned();
            if diags.trim().is_empty() {
                anyhow::bail!("{err}");
            }
            anyhow::bail!("{err}\n{diags}");
        },
    };

    Ok(units
        .into_iter()
        .map(|unit| {
            let is_script = matches!(unit, CompiledUnitEnum::Script(_));
            let bytecode = unit.into_compiled_unit().serialize(Some(bytecode_version));
            CompiledArtifact {
                is_script,
                bytecode,
            }
        })
        .collect())
}

/// Parse the JSON named-address mapping argument shared by the compile bindings.
///
/// Accepts an empty string or a JSON object of `{ "name": "0xADDR", ... }`.
fn parse_named_addresses(json: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let json = json.trim();
    if json.is_empty() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_str(json)
        .map_err(|e| anyhow::anyhow!("failed to parse named addresses JSON: {e}"))
}

/// Compile a single self-contained source string, returning the bytecode of the
/// module or script it defines (serialized at bytecode v10).
fn compile_single(
    source: &str,
    named_addresses_json: &str,
    want_script: bool,
) -> anyhow::Result<Vec<u8>> {
    let named_addresses = parse_named_addresses(named_addresses_json)?;
    let file_name = if want_script {
        "script.move"
    } else {
        "module.move"
    };
    let artifacts = compile_impl(
        file_name,
        source,
        &named_addresses,
        &BTreeMap::new(),
        move_binary_format::file_format_common::VERSION_10,
    )?;
    match artifacts.into_iter().find(|a| a.is_script == want_script) {
        Some(artifact) => Ok(artifact.bytecode),
        None => anyhow::bail!(
            "source did not produce a {}",
            if want_script { "script" } else { "module" }
        ),
    }
}

/// Compile Move module source code to bytecode (bytecode v10).
///
/// # Arguments
/// * `source` - Move source text defining exactly one module
/// * `named_addresses_json` - JSON object mapping named addresses to hex
///   addresses, e.g. `{"my_addr":"0x42"}`. May be empty (`""`) if the source
///   only uses numeric addresses.
///
/// # Returns
/// Serialized module bytecode (a `.mv` byte array) at bytecode version 10.
///
/// # Errors
/// Returns an error (with compiler diagnostics) if compilation fails or the
/// source does not define a module.
#[wasm_bindgen]
pub fn compile_module(source: &str, named_addresses_json: &str) -> Result<Vec<u8>> {
    compile_single(source, named_addresses_json, false).map_err(to_js_error)
}

/// Compile Move script source code to bytecode (bytecode v10).
///
/// # Arguments
/// * `source` - Move source text defining a script (a `script { ... }` block)
/// * `named_addresses_json` - JSON object mapping named addresses to hex
///   addresses. May be empty (`""`).
///
/// # Returns
/// Serialized script bytecode at bytecode version 10.
#[wasm_bindgen]
pub fn compile_script(source: &str, named_addresses_json: &str) -> Result<Vec<u8>> {
    compile_single(source, named_addresses_json, true).map_err(to_js_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_binary_format::file_format::{
        basic_test_module, basic_test_script, empty_module,
    };
    use move_binary_format::file_format_common::{
        VERSION_10, VERSION_5, VERSION_DEFAULT, VERSION_MAX,
    };

    // NOTE: The public `#[wasm_bindgen]` entry points return `JsValue` errors,
    // and constructing a `JsValue` aborts on non-wasm hosts. The unit tests
    // therefore exercise the target-independent `*_impl` helpers, which carry
    // the actual decompilation/disassembly logic. The thin wasm wrappers and
    // JS interop are covered by the `wasm32` build itself.

    /// Serialize a known-good module at the requested bytecode version.
    fn module_bytes_at_version(version: u32) -> Vec<u8> {
        let module = basic_test_module();
        let mut bytes = Vec::new();
        module
            .serialize_for_version(Some(version), &mut bytes)
            .expect("module should serialize");
        bytes
    }

    /// Serialize a known-good script at the requested bytecode version.
    fn script_bytes_at_version(version: u32) -> Vec<u8> {
        let script = basic_test_script();
        let mut bytes = Vec::new();
        script
            .serialize_for_version(Some(version), &mut bytes)
            .expect("script should serialize");
        bytes
    }

    #[test]
    fn test_version_info() {
        let info = get_version_info();
        assert!(info.contains("move-decompiler-wasm"));
        assert!(info.contains("version"));
        assert!(info.contains("disassemble_module"));
        assert!(info.contains("decompile_module"));
    }

    /// The decompiler/disassembler must accept the latest bytecode format (v10).
    #[test]
    fn test_supports_bytecode_v10() {
        // Guard (at compile time) against the upstream max version regressing
        // below what we claim to support.
        const { assert!(VERSION_MAX >= VERSION_10, "upstream must support v10") };

        let bytes = module_bytes_at_version(VERSION_10);

        // Deserialization accepts v10.
        assert!(deserialize_module(&bytes).is_ok());

        // Metadata correctly reports the v10 version.
        let metadata = module_metadata_impl(&bytes).expect("metadata for v10 module");
        assert_eq!(metadata.version(), VERSION_10);
        assert!(metadata.function_count() >= 1);
        assert!(metadata.struct_count() >= 1);
    }

    #[test]
    fn test_disassemble_v10_module() {
        let bytes = module_bytes_at_version(VERSION_10);
        let asm = disassemble_module_impl(&bytes).expect("disassembly of v10 module");
        assert!(!asm.is_empty());
        // The disassembler should surface the module's function name.
        assert!(asm.contains("foo"), "disassembly should mention `foo`: {asm}");
    }

    #[test]
    fn test_decompile_v10_module() {
        let bytes = module_bytes_at_version(VERSION_10);
        let source = decompile_module_impl(&bytes).expect("decompilation of v10 module");
        assert!(!source.is_empty());
        assert!(
            source.contains("module"),
            "decompiled output should contain a module declaration: {source}"
        );
    }

    /// Disassembly and metadata extraction should work across the full range of
    /// supported bytecode versions, including v10.
    #[test]
    fn test_all_supported_versions_roundtrip() {
        for version in VERSION_5..=VERSION_10 {
            let bytes = module_bytes_at_version(version);
            assert!(
                deserialize_module(&bytes).is_ok(),
                "v{version} bytecode should deserialize"
            );
            let metadata =
                module_metadata_impl(&bytes).unwrap_or_else(|_| panic!("metadata for v{version}"));
            assert_eq!(metadata.version(), version);
            assert!(
                disassemble_module_impl(&bytes).is_ok(),
                "v{version} bytecode should disassemble"
            );
        }
    }

    #[test]
    fn test_default_version_is_v10() {
        // The crate is expected to be built against an upstream where v10 is the
        // default serialization version (checked at compile time).
        const { assert!(VERSION_DEFAULT == VERSION_10, "v10 must be the default version") };

        let module = empty_module();
        let mut bytes = Vec::new();
        module.serialize(&mut bytes).expect("serialize default");
        let metadata = module_metadata_impl(&bytes).expect("metadata for default module");
        assert_eq!(metadata.version(), VERSION_10);
    }

    /// The disassembler/decompiler must accept the latest bytecode format (v10)
    /// for scripts as well as modules.
    #[test]
    fn test_supports_bytecode_v10_script() {
        let bytes = script_bytes_at_version(VERSION_10);

        // Deserialization accepts a v10 script and reports the v10 version.
        let script = deserialize_script(&bytes).expect("v10 script should deserialize");
        assert_eq!(script.version, VERSION_10);

        // The script disassembles and decompiles without error.
        let asm = disassemble_script_impl(&bytes).expect("disassembly of v10 script");
        assert!(!asm.is_empty(), "v10 script disassembly should be non-empty");

        let source = decompile_script_impl(&bytes).expect("decompilation of v10 script");
        assert!(
            !source.is_empty(),
            "v10 script decompilation should be non-empty"
        );
    }

    /// Script disassembly should work across the full range of supported
    /// bytecode versions, including v10.
    #[test]
    fn test_all_supported_script_versions_roundtrip() {
        for version in VERSION_5..=VERSION_10 {
            let bytes = script_bytes_at_version(version);
            let script = deserialize_script(&bytes)
                .unwrap_or_else(|_| panic!("v{version} script should deserialize"));
            assert_eq!(script.version, version);
            assert!(
                disassemble_script_impl(&bytes).is_ok(),
                "v{version} script should disassemble"
            );
        }
    }

    /// Compiling module source (in-memory, no filesystem) should yield bytecode
    /// that round-trips through the v10 deserializer/disassembler.
    #[test]
    fn test_compile_module_v10() {
        let source = r#"
            module 0x42::example {
                public fun add(x: u64, y: u64): u64 {
                    x + y
                }
            }
        "#;
        let bytes = compile_single(source, "", false).expect("module should compile");

        // Deserializes and reports v10.
        let metadata = module_metadata_impl(&bytes).expect("metadata for compiled module");
        assert_eq!(metadata.version(), VERSION_10);
        assert!(metadata.function_count() >= 1);

        // The freshly compiled module round-trips through the disassembler.
        let asm = disassemble_module_impl(&bytes).expect("disassemble compiled module");
        assert!(
            asm.contains("add") || asm.contains("example"),
            "disassembly should mention the compiled code: {asm}"
        );
    }

    /// Compiling with a named-address mapping should resolve the address.
    #[test]
    fn test_compile_module_with_named_address() {
        let source = "module addr::m { public fun f(): u64 { 1 } }";
        let bytes =
            compile_single(source, "{\"addr\":\"0x99\"}", false).expect("module should compile");
        let metadata = module_metadata_impl(&bytes).expect("metadata for compiled module");
        assert_eq!(metadata.version(), VERSION_10);
        assert!(metadata.function_count() >= 1);
    }

    /// Compiling script source should yield a v10 script.
    #[test]
    fn test_compile_script_v10() {
        let source = r#"
            script {
                fun main() { }
            }
        "#;
        let bytes = compile_single(source, "", true).expect("script should compile");
        let script = deserialize_script(&bytes).expect("compiled script deserializes");
        assert_eq!(script.version, VERSION_10);
        assert!(disassemble_script_impl(&bytes).is_ok());
    }

    #[test]
    fn test_compile_rejects_invalid_source() {
        let bad = "module 0x1::m { this is not valid move }";
        assert!(compile_single(bad, "", false).is_err());
    }

    #[test]
    fn test_rejects_invalid_bytecode() {
        let garbage = [0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02];
        assert!(deserialize_module(&garbage).is_err());
        assert!(disassemble_module_impl(&garbage).is_err());
        assert!(decompile_module_impl(&garbage).is_err());
        assert!(module_metadata_impl(&garbage).is_err());
        assert!(deserialize_script(&garbage).is_err());
        assert!(disassemble_script_impl(&garbage).is_err());
        assert!(decompile_script_impl(&garbage).is_err());
    }
}
