// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Thread-local source-location registry for the Move VM debugger.
//!
//! The VM runtime has no dependency on Aptos-specific packages; source maps are
//! stored in on-chain metadata that only the Aptos layer can access.  This
//! module provides a type-erased trait so the Aptos layer can register a
//! provider once per replay and the deep interpreter code can call it without
//! introducing a crate dependency on Aptos.

use move_binary_format::{errors::PartialVMResult, file_format::FunctionDefinitionIndex};
use move_core_types::language_storage::ModuleId;
use move_vm_types::values::Locals;
use std::{cell::RefCell, fmt, sync::Arc};

// ── Public trait ─────────────────────────────────────────────────────────────

/// Provides source-level information that is only available to the Aptos layer.
///
/// Implementors live in `aptos-move/aptos-debugger`; the VM runtime only holds
/// a `dyn SourceLocator` behind an `Arc`.
pub trait SourceLocator: Send + Sync {
    /// Return `"filename:line"` for the bytecode position `pc` inside the
    /// function identified by `func_def_idx` of `module_id`, or `None` when
    /// no source map is available for that function.
    fn get_bytecode_source_location(
        &self,
        module_id: &ModuleId,
        func_def_idx: FunctionDefinitionIndex,
        pc: u16,
    ) -> Option<String>;

    /// Return `(param_count, names)` where `names` is the concatenation of
    /// parameter names followed by local-variable names for the given function.
    /// Returns `None` when no source map is available.
    fn get_function_param_and_local_names(
        &self,
        module_id: &ModuleId,
        func_def_idx: FunctionDefinitionIndex,
    ) -> Option<(usize, Vec<String>)>;

    /// Return the ordered list of field names for the struct `struct_name`
    /// defined in `module_id`, or `None` when no information is available.
    fn get_struct_field_names(
        &self,
        module_id: &ModuleId,
        struct_name: &str,
    ) -> Option<Vec<String>>;

    /// Return `(variant_name, field_names)` pairs for each variant of enum
    /// `enum_name` in `module_id`, indexed by variant tag (0-based), or
    /// `None` when the type is not a known enum.
    fn get_enum_variant_info(
        &self,
        module_id: &ModuleId,
        enum_name: &str,
    ) -> Option<Vec<(String, Vec<String>)>>;
}

// ── Thread-local storage ─────────────────────────────────────────────────────

thread_local! {
    static LOCATOR: RefCell<Option<Arc<dyn SourceLocator>>> = RefCell::new(None);
}

/// Install `loc` as the source locator for the current thread, replacing any
/// previous one.  Call [`clear_source_locator`] after replay finishes to avoid
/// stale state on thread-pool threads.
pub fn set_source_locator(loc: Arc<dyn SourceLocator>) {
    LOCATOR.with(|l| *l.borrow_mut() = Some(loc));
}

/// Remove the source locator for the current thread.
pub fn clear_source_locator() {
    LOCATOR.with(|l| *l.borrow_mut() = None);
}

// ── Accessor helpers (called from interpreter / debug loop) ──────────────────

/// Query the current thread's source locator for a `"file:line"` string.
pub fn get_bytecode_source_location(
    module_id: &ModuleId,
    func_def_idx: FunctionDefinitionIndex,
    pc: u16,
) -> Option<String> {
    LOCATOR.with(|l| {
        l.borrow()
            .as_ref()
            .and_then(|loc| loc.get_bytecode_source_location(module_id, func_def_idx, pc))
    })
}

/// Query the current thread's source locator for parameter / local names.
pub fn get_function_param_and_local_names(
    module_id: &ModuleId,
    func_def_idx: FunctionDefinitionIndex,
) -> Option<(usize, Vec<String>)> {
    LOCATOR.with(|l| {
        l.borrow()
            .as_ref()
            .and_then(|loc| loc.get_function_param_and_local_names(module_id, func_def_idx))
    })
}

/// Query the current thread's source locator for struct field names.
pub fn get_struct_field_names(module_id: &ModuleId, struct_name: &str) -> Option<Vec<String>> {
    LOCATOR.with(|l| {
        l.borrow()
            .as_ref()
            .and_then(|loc| loc.get_struct_field_names(module_id, struct_name))
    })
}

/// Query the current thread's source locator for enum variant info.
pub fn get_enum_variant_info(
    module_id: &ModuleId,
    enum_name: &str,
) -> Option<Vec<(String, Vec<String>)>> {
    LOCATOR.with(|l| {
        l.borrow()
            .as_ref()
            .and_then(|loc| loc.get_enum_variant_info(module_id, enum_name))
    })
}

/// Write enriched local-variable display (with source names and struct field
/// names when a source locator is registered) into `buf`.
///
/// Callers are responsible for the empty-locals guard and any surrounding
/// blank lines; this function only writes the Parameters / Locals sections.
///
/// `compact` – when `true`, `Invalid` slots (already moved out) are omitted.
pub(crate) fn print_locals_enriched<B: fmt::Write>(
    buf: &mut B,
    function: &crate::LoadedFunction,
    locals: &Locals,
    runtime_environment: &crate::RuntimeEnvironment,
    compact: bool,
) -> PartialVMResult<()> {
    let names_info = function
        .module_id()
        .and_then(|mid| get_function_param_and_local_names(mid, function.index()));
    let param_count = function.param_tys().len();

    if let Some((_, ref names)) = names_info {
        let type_info: Vec<Option<move_vm_types::values::LocalTypeInfo>> = function
            .local_tys()
            .iter()
            .map(|ty| {
                let (mid, sname) = runtime_environment.get_struct_name(ty).ok().flatten()?;
                let sname_str = sname.as_str();
                if let Some(fields) = get_struct_field_names(&mid, sname_str) {
                    Some(move_vm_types::values::LocalTypeInfo::Struct(fields))
                } else {
                    get_enum_variant_info(&mid, sname_str)
                        .map(move_vm_types::values::LocalTypeInfo::Enum)
                }
            })
            .collect();
        move_vm_types::values::debug::print_locals_with_info(
            buf,
            locals,
            compact,
            param_count,
            Some(names.as_slice()),
            Some(type_info.as_slice()),
        )
    } else {
        move_vm_types::values::debug::print_locals_with_info(
            buf,
            locals,
            compact,
            param_count,
            None,
            None,
        )
    }
}
