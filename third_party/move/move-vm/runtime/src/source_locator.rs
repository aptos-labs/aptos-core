// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Thread-local source-location registry for the Move VM debugger.
//!
//! The VM runtime has no dependency on Aptos-specific packages; source maps are
//! stored in on-chain metadata that only the Aptos layer can access.  This
//! module provides a type-erased trait so the Aptos layer can register a
//! provider once per replay and the deep interpreter code can call it without
//! introducing a crate dependency on Aptos.

use crate::RuntimeEnvironment;
use move_binary_format::{errors::PartialVMResult, file_format::FunctionDefinitionIndex};
use move_core_types::language_storage::ModuleId;
use move_vm_types::{
    debug_write,
    loaded_data::runtime_types::Type,
    values::{debug, debug::DebugValue, Locals, Value},
};
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

/// Return a clone of the current thread's source locator (if any).
pub fn get_source_locator() -> Option<Arc<dyn SourceLocator>> {
    LOCATOR.with(|l| l.borrow().clone())
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

#[derive(Debug)]
pub struct LocalInfo {
    pub index: usize,
    pub name: String,
}

/// Resolve local variable info for a function using the global source
/// locator. Returns one `LocalInfo` per local slot (parameters + locals).
/// When no source map is available, names fall back to `local[idx]`.
pub(crate) fn build_local_infos(function: &crate::LoadedFunction) -> Vec<LocalInfo> {
    let total = function.local_tys().len();

    let names = function
        .module_id()
        .and_then(|mid| get_function_param_and_local_names(mid, function.index()))
        .map(|(_, n)| n);

    (0..total)
        .map(|idx| {
            let name = names
                .as_ref()
                .and_then(|n| n.get(idx).cloned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("local[{}]", idx));
            LocalInfo { index: idx, name }
        })
        .collect()
}

pub struct DebugTypeNameResolver<'a> {
    runtime_environment: &'a RuntimeEnvironment,
    interpreter: &'a dyn crate::interpreter::InterpreterDebugInterface,
}

impl<'a> DebugTypeNameResolver<'a> {
    pub(crate) fn new(
        runtime_environment: &'a RuntimeEnvironment,
        interpreter: &'a dyn crate::interpreter::InterpreterDebugInterface,
    ) -> Self {
        Self {
            runtime_environment,
            interpreter,
        }
    }
}

impl debug::TypeNameResolver for DebugTypeNameResolver<'_> {
    fn get_struct_field_names(&self, ty: &Type) -> Option<Vec<String>> {
        let (mid, sname) = self
            .runtime_environment
            .get_struct_name(ty)
            .ok()
            .flatten()?;
        get_struct_field_names(&mid, sname.as_str())
    }

    fn get_enum_variant_info(&self, ty: &Type) -> Option<Vec<(String, Vec<String>)>> {
        let (mid, sname) = self
            .runtime_environment
            .get_struct_name(ty)
            .ok()
            .flatten()?;
        get_enum_variant_info(&mid, sname.as_str())
    }

    fn load_struct_type(
        &self,
        ty: &Type,
    ) -> Option<Arc<move_vm_types::loaded_data::runtime_types::StructType>> {
        match ty {
            Type::Struct { idx, .. } | Type::StructInstantiation { idx, .. } => {
                self.interpreter.load_struct_type(idx)
            },
            _ => None,
        }
    }
}

struct ZeroNameResolver;
impl debug::TypeNameResolver for ZeroNameResolver {
    fn get_struct_field_names(&self, _ty: &Type) -> Option<Vec<String>> {
        None
    }

    fn get_enum_variant_info(&self, _ty: &Type) -> Option<Vec<(String, Vec<String>)>> {
        None
    }

    fn load_struct_type(
        &self,
        _ty: &Type,
    ) -> Option<Arc<move_vm_types::loaded_data::runtime_types::StructType>> {
        None
    }
}

pub fn print_value(buf: &mut impl fmt::Write, val: &Value) -> PartialVMResult<()> {
    let debug_value = debug::serialize_value(val, None, &ZeroNameResolver);
    debug_write!(buf, "{}", debug_value)
}

/// Write enriched local-variable display (with source names and struct field
/// names when a source locator is registered) into `buf`.
pub(crate) fn print_locals_enriched<B: fmt::Write>(
    buf: &mut B,
    function: &crate::LoadedFunction,
    locals: &Locals,
    runtime_environment: &RuntimeEnvironment,
    interpreter: &dyn crate::interpreter::InterpreterDebugInterface,
    compact: bool,
) -> PartialVMResult<()> {
    use move_vm_types::{debug_write, debug_writeln, values::debug::serialize_value_for_debug};

    let infos = build_local_infos(function);
    let name_resolver = DebugTypeNameResolver::new(runtime_environment, interpreter);
    let param_count = function.param_tys().len();
    let total = infos.len();

    let mut printed_header = false;
    for info in &infos[..param_count.min(total)] {
        let ty = &function.local_tys()[info.index];
        let dv = serialize_value_for_debug(locals, info.index, ty, &name_resolver);
        if compact && matches!(&dv, DebugValue::Invalid) {
            continue;
        }
        if !printed_header {
            debug_writeln!(buf, "        Parameters:")?;
            printed_header = true;
        }
        debug_write!(buf, "            [{}] {}: ", info.index, info.name)?;
        debug_write!(buf, "{}", dv)?;
        debug_writeln!(buf)?;
    }

    printed_header = false;
    for info in &infos[param_count..total] {
        let ty = &function.local_tys()[info.index];
        let dv = serialize_value_for_debug(locals, info.index, ty, &name_resolver);
        if compact && matches!(&dv, DebugValue::Invalid) {
            continue;
        }
        if !printed_header {
            debug_writeln!(buf, "        Locals:")?;
            printed_header = true;
        }
        debug_write!(buf, "            [{}] {}: ", info.index, info.name)?;
        debug_write!(buf, "{}", dv)?;
        debug_writeln!(buf)?;
    }

    Ok(())
}
