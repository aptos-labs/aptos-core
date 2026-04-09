// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Aptos-specific implementation of [`SourceLocator`] for the Move VM debugger.
//!
//! [`AptosSourceLocator`] holds pre-parsed source maps and struct field name
//! tables built from on-chain `PackageMetadata` and/or locally-compiled
//! packages.  It is registered on the thread before a replay begins and
//! cleared afterwards so that thread-pool threads don't carry stale data.

use ahash::AHashMap;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionDefinitionIndex, StructFieldInformation},
    CompiledModule,
};
use move_bytecode_source_map::source_map::SourceMap;
use move_command_line_common::files::FileHash;
use move_core_types::language_storage::ModuleId;
use move_vm_runtime::source_locator::SourceLocator;
use std::sync::OnceLock;

/// Set to any value to print why source lines couldn't be resolved for a
/// given module (missing source map, unrecognised file hash, etc.).
const MOVE_SOURCE_LOCATOR_DEBUG_ENV_VAR: &str = "MOVE_SOURCE_LOCATOR_DEBUG";

static SOURCE_LOCATOR_DEBUG: OnceLock<bool> = OnceLock::new();

fn is_debug_enabled() -> bool {
    *SOURCE_LOCATOR_DEBUG.get_or_init(|| std::env::var(MOVE_SOURCE_LOCATOR_DEBUG_ENV_VAR).is_ok())
}

// ── Internal data structures ─────────────────────────────────────────────────

struct FileData {
    filename: String,
    /// Byte offsets of the start of each line (index 0 → byte 0).
    line_starts: Vec<u32>,
}

struct ModuleSourceData {
    source_map: SourceMap,
    files: AHashMap<FileHash, FileData>,
}

// ── Public struct ─────────────────────────────────────────────────────────────

/// Aptos implementation of the [`SourceLocator`] trait.
///
/// Build one with [`AptosSourceLocator::new`], populate it with
/// [`add_package_at_address`](Self::add_package_at_address) for on-chain
/// packages or [`add_compiled_module`](Self::add_compiled_module) for local
/// packages, then call
/// `move_vm_runtime::source_locator::set_source_locator(Arc::new(locator))`.
pub struct AptosSourceLocator {
    source_data: AHashMap<ModuleId, ModuleSourceData>,
    struct_field_names: AHashMap<ModuleId, AHashMap<String, Vec<String>>>,
    enum_variants: AHashMap<ModuleId, AHashMap<String, Vec<(String, Vec<String>)>>>,
}

impl Default for AptosSourceLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl AptosSourceLocator {
    pub fn new() -> Self {
        Self {
            source_data: AHashMap::new(),
            struct_field_names: AHashMap::new(),
            enum_variants: AHashMap::new(),
        }
    }

    // ── Population helpers ────────────────────────────────────────────────────

    /// Add source-map and struct field-name data from a locally-compiled module.
    ///
    /// `source_map_bcs` – raw (non-compressed) BCS-encoded [`SourceMap`],
    ///   as produced by `compiled_unit.serialize_source_map()`.
    /// `source_text` – the content of the `.move` source file.
    /// `source_filename` – the display filename.
    pub fn add_local_module(
        &mut self,
        module: &CompiledModule,
        source_map_bcs: &[u8],
        source_text: &str,
        source_filename: &str,
    ) -> anyhow::Result<()> {
        let module_id = module.self_id();

        // Parse source map (raw BCS, not compressed for local builds).
        let source_map: SourceMap = bcs::from_bytes(source_map_bcs)?;

        let file_hash = FileHash::new(source_text);
        let line_starts = build_line_starts(source_text);
        let mut files = AHashMap::new();
        files.insert(file_hash, FileData {
            filename: source_filename.to_owned(),
            line_starts,
        });

        self.source_data
            .insert(module_id.clone(), ModuleSourceData { source_map, files });

        // Extract struct field names from the compiled module.
        self.add_struct_field_names_from_module(module);

        Ok(())
    }

    /// Extract struct and enum type information from a compiled module and store
    /// it for annotated local-variable display in the debugger.
    pub fn add_struct_field_names_from_module(&mut self, compiled_module: &CompiledModule) {
        let module_id = compiled_module.self_id();

        for struct_def in compiled_module.struct_defs() {
            let handle = compiled_module.struct_handle_at(struct_def.struct_handle);
            let struct_name = compiled_module.identifier_at(handle.name).to_string();

            match &struct_def.field_information {
                StructFieldInformation::Native => {},
                StructFieldInformation::Declared(fields) => {
                    let field_names = fields
                        .iter()
                        .map(|f| compiled_module.identifier_at(f.name).to_string())
                        .collect();
                    self.struct_field_names
                        .entry(module_id.clone())
                        .or_default()
                        .insert(struct_name, field_names);
                },
                StructFieldInformation::DeclaredVariants(variants) => {
                    let variant_info = variants
                        .iter()
                        .map(|v| {
                            let name = compiled_module.identifier_at(v.name).to_string();
                            let fields = v
                                .fields
                                .iter()
                                .map(|f| compiled_module.identifier_at(f.name).to_string())
                                .collect();
                            (name, fields)
                        })
                        .collect();
                    self.enum_variants
                        .entry(module_id.clone())
                        .or_default()
                        .insert(struct_name, variant_info);
                },
            }
        }
    }
}

// ── SourceLocator implementation ──────────────────────────────────────────────

impl SourceLocator for AptosSourceLocator {
    fn get_bytecode_source_location(
        &self,
        module_id: &ModuleId,
        func_def_idx: FunctionDefinitionIndex,
        pc: u16,
    ) -> Option<String> {
        let debug = is_debug_enabled();

        let data = match self.source_data.get(module_id) {
            Some(d) => d,
            None => {
                if debug {
                    eprintln!("[source_locator] no source data for module {}", module_id);
                }
                return None;
            },
        };
        let func_map = match data.source_map.get_function_source_map(func_def_idx) {
            Ok(m) => m,
            Err(e) => {
                if debug {
                    eprintln!(
                        "[source_locator] no function source map for {}#{}: {}",
                        module_id, func_def_idx.0, e
                    );
                }
                return None;
            },
        };
        let loc = match func_map.get_code_location(pc) {
            Some(l) => l,
            None => {
                if debug {
                    eprintln!(
                        "[source_locator] no code location for {}#{} pc={}",
                        module_id, func_def_idx.0, pc
                    );
                }
                return None;
            },
        };
        let file_data = match data.files.get(&loc.file_hash()) {
            Some(f) => f,
            None => {
                if debug {
                    eprintln!("[source_locator] source file not found for {}", module_id);
                }
                return None;
            },
        };
        let line = byte_offset_to_line(&file_data.line_starts, loc.start());
        Some(format!("{}:{}", file_data.filename, line + 1))
    }

    fn get_function_param_and_local_names(
        &self,
        module_id: &ModuleId,
        func_def_idx: FunctionDefinitionIndex,
    ) -> Option<(usize, Vec<String>)> {
        let data = self.source_data.get(module_id)?;
        let func_map = data.source_map.get_function_source_map(func_def_idx).ok()?;
        let param_count = func_map.parameters.len();
        let names = func_map
            .parameters
            .iter()
            .chain(func_map.locals.iter())
            .map(|sn| sn.0.clone())
            .collect();
        Some((param_count, names))
    }

    fn get_struct_field_names(
        &self,
        module_id: &ModuleId,
        struct_name: &str,
    ) -> Option<Vec<String>> {
        self.struct_field_names
            .get(module_id)?
            .get(struct_name)
            .cloned()
    }

    fn get_enum_variant_info(
        &self,
        module_id: &ModuleId,
        enum_name: &str,
    ) -> Option<Vec<(String, Vec<String>)>> {
        self.enum_variants.get(module_id)?.get(enum_name).cloned()
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// Build a sorted vector of byte offsets at which each line begins.
/// Index 0 is always 0 (line 1 starts at byte 0).
fn build_line_starts(text: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, _) in text.match_indices('\n') {
        starts.push(i as u32 + 1);
    }
    starts
}

/// Convert a byte offset into a 0-based line number using binary search on
/// the pre-built `line_starts` array.
fn byte_offset_to_line(line_starts: &[u32], byte_offset: u32) -> u32 {
    match line_starts.binary_search(&byte_offset) {
        Ok(i) => i as u32,
        Err(i) => (i.saturating_sub(1)) as u32,
    }
}
