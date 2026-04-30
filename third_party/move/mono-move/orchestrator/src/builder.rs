// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Builds a [`LoadedModule`] from a [`PreparedModule`] by resolving its
//! struct/enum types, running the specializer, and packaging the result
//! alongside the polymorphic IR.
//!
//! The abstract interning interface (`Interner`, `StructResolver`,
//! `walk_sig_token`) lives in `mono-move-core`; the concrete implementation
//! that owns the global tables is provided by `ExecutionGuard` in
//! `mono-move-global-context`. This module drives the module-level walk
//! (struct defs, enum defs, layout computation), delegates leaf type
//! interning to the guard, and assembles the final `LoadedModule`.

use anyhow::{anyhow, bail, Result};
use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    types::{align_up, view_type, Type},
    EnumType, Executable, ExecutableId, FrameLayoutInfo, Function, MicroOp, PreparedModule,
    SortedSafePointEntries, StructType, VariantFields,
};
use mono_move_global_context::{
    ExecutionGuard, FieldLayout, InternedType, LoadedModule, MandatoryDependencies,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionHandleIndex, StructDefinitionIndex, StructFieldInformation},
    CompiledModule,
};
use shared_dsa::UnorderedMap;
use specializer::lower_module;
use std::ops::Deref;

// TODO: this is likely to change. Placeholder.
// Split mutable state into a separate struct to avoid borrow conflicts with self.module.
#[allow(dead_code)]
pub struct ExecutableBuilder<'guard, 'ctx> {
    // TODO: support scripts.
    module: PreparedModule,
    /// Maps interned struct/enum names to their `StructDefinitionIndex`
    /// for local defs. Used by the layout pass to recurse into dependent
    /// local defs when a field type points at another local struct/enum.
    local_def_by_name: UnorderedMap<GlobalArenaPtr<str>, usize>,

    /// Executable ID.
    id: GlobalArenaPtr<ExecutableId>,
    /// Deterministic load cost recorded on the built executable.
    cost: u64,
    /// Mandatory-dependency descriptor recorded on the built executable.
    mandatory_dependencies: MandatoryDependencies,
    /// Non-generic struct definitions being built.
    structs: UnorderedMap<GlobalArenaPtr<str>, StructType>,
    /// Non-generic enum definitions being built.
    enums: UnorderedMap<GlobalArenaPtr<str>, EnumType>,
    /// Non-generic functions being built.
    functions: UnorderedMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>>,
    /// Function pointers indexed by `FunctionHandleIndex`, for call
    /// patching. Local definitions fill entries; cross-module handles stay
    /// `None`.
    func_ptrs: Vec<Option<ExecutableArenaPtr<Function>>>,
    /// Stores all allocations for this executable.
    arena: ExecutableArena,
    /// Context for interning.
    guard: &'guard ExecutionGuard<'ctx>,
}

impl<'guard, 'ctx> ExecutableBuilder<'guard, 'ctx> {
    /// Creates a new builder for transforming `module` into an [`Executable`].
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, module: CompiledModule) -> Result<Self>
    where
        'ctx: 'guard,
    {
        let module = PreparedModule::build(module, guard)?;
        let local_def_by_name = module
            .struct_defs()
            .iter()
            .enumerate()
            .map(|(i, def)| {
                let handle = module.struct_handle_at(def.struct_handle);
                let name = guard
                    .intern_identifier(module.identifier_at(handle.name))
                    .into_global_arena_ptr();
                (name, i)
            })
            .collect::<UnorderedMap<_, _>>();

        let id = guard
            .intern_address_name(module.self_addr(), module.self_name())
            .into_global_arena_ptr();
        let num_func_handles = module.function_handles.len();

        Ok(ExecutableBuilder {
            module,
            local_def_by_name,
            id,
            cost: 0,
            mandatory_dependencies: MandatoryDependencies::empty(),
            structs: UnorderedMap::new(),
            enums: UnorderedMap::new(),
            functions: UnorderedMap::new(),
            func_ptrs: vec![None; num_func_handles],
            arena: ExecutableArena::new(),
            guard,
        })
    }

    /// Sets the deterministic load cost for the built executable.
    pub fn with_cost(mut self, cost: u64) -> Self {
        self.cost = cost;
        self
    }

    /// Sets the mandatory-dependency descriptor for the built executable.
    /// The contained slots must exclude this executable's own slot.
    pub fn with_mandatory_dependencies(mut self, deps: MandatoryDependencies) -> Self {
        self.mandatory_dependencies = deps;
        self
    }

    /// Computes layouts for every local struct, registers every local enum,
    /// and populates `self.structs`/`self.enums`. Must be called before
    /// `finish()`.
    pub fn resolve_types(&mut self) -> Result<()> {
        for def_idx in 0..self.module.struct_defs.len() {
            self.ensure_def(def_idx)?;
        }
        Ok(())
    }

    /// Runs the full build pipeline (resolve types → destack → lower every
    /// function → resolve calls → assemble `LoadedModule`).
    pub fn build(mut self) -> Result<Box<LoadedModule>> {
        // TODO: this clone is needed is because we need to resolve layouts.
        // this will be gone once layout construction is refactored into its own pass.
        let module_ir = specializer::destack(self.module.deref().clone(), self.guard)?;
        self.resolve_types()?;
        let lowered = lower_module(&module_ir)?;
        let module = &module_ir.module;

        // Record each lowered function in the executable arena. Inlined
        // here (not a `&mut self` method) because `self.module` has been
        // partially moved into `destack` above, which forbids `&mut self`
        // borrows; direct field access on `self.arena` etc. is fine.
        for lowered_fn in lowered.functions {
            let name = self
                .guard
                .intern_identifier(module.identifier_at(lowered_fn.name_idx))
                .into_global_arena_ptr();
            let code = self.arena.alloc_slice_fill_iter(lowered_fn.code);
            let param_sizes = self.arena.alloc_slice_fill_iter(lowered_fn.param_sizes);
            let func = Function {
                name,
                code,
                param_sizes,
                param_sizes_sum: lowered_fn.param_sizes_sum,
                param_and_local_sizes_sum: lowered_fn.param_and_local_sizes_sum,
                extended_frame_size: lowered_fn.extended_frame_size,
                // TODO: hardcoded for now.
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            };
            let ptr = self.arena.alloc(func);
            self.functions.insert(name, ptr);
            self.func_ptrs[lowered_fn.handle_idx.0 as usize] = Some(ptr);
        }

        // Rewrite every `CallFunc` in every local function's code:
        // - Handles resolving to a local `Function` pointer become `CallDirect`.
        // - Cross-module handles become `CallIndirect` keyed by the callee's
        //   interned `(executable_id, func_name)` pair, dispatched at runtime.
        // Inlined for the same partial-move reason as the loop above.
        let self_module_handle_idx = module.self_module_handle_idx;
        for func_ptr in &self.func_ptrs {
            let Some(mut func_ptr) = *func_ptr else {
                continue;
            };
            // SAFETY: We have exclusive access during build — no concurrent
            // readers exist yet. The arena is alive because `self` still
            // owns it until `Executable::new` is called below.
            let func = unsafe { func_ptr.as_mut_unchecked() };
            let code = unsafe { func.code.as_mut_unchecked() };
            for op in code.iter_mut() {
                let MicroOp::CallFunc { func_id } = *op else {
                    continue;
                };
                if let Some(ptr) = self.func_ptrs[func_id as usize] {
                    *op = MicroOp::CallDirect { ptr };
                    continue;
                }
                let callee_handle = module.function_handle_at(FunctionHandleIndex(func_id as u16));
                if callee_handle.module == self_module_handle_idx {
                    bail!("unresolved local function handle {}", func_id);
                }
                let callee_module = module.module_handle_at(callee_handle.module);
                let executable_id = self
                    .guard
                    .intern_address_name(
                        module.address_identifier_at(callee_module.address),
                        module.identifier_at(callee_module.name),
                    )
                    .into_global_arena_ptr();
                let func_name = self
                    .guard
                    .intern_identifier(module.identifier_at(callee_handle.name))
                    .into_global_arena_ptr();
                *op = MicroOp::CallIndirect {
                    executable_id,
                    func_name,
                };
            }
        }

        let executable = Executable::new(
            self.id,
            self.cost,
            self.structs,
            self.enums,
            self.functions,
            self.arena,
        );
        Ok(LoadedModule::new(
            module_ir,
            executable,
            self.mandatory_dependencies,
        ))
    }
}

//
// Only private APIs below.
// ------------------------

impl<'guard, 'ctx> ExecutableBuilder<'guard, 'ctx> {
    /// Ensures `def_idx`'s struct or enum has been processed: layout
    /// computed (for structs) or `EnumType` registered (for variant defs),
    /// with all locally-defined dependencies recursively processed first.
    /// Idempotent: short-circuits if `self.structs`/`self.enums` already
    /// contains an entry for this def's name.
    fn ensure_def(&mut self, def_idx: usize) -> Result<()> {
        let struct_handle = self.module.struct_defs[def_idx].struct_handle;
        let handle = self.module.struct_handle_at(struct_handle);
        if !handle.type_parameters.is_empty() {
            bail!("Generic structs / enums not yet supported");
        }
        let name = self
            .guard
            .intern_identifier(self.module.identifier_at(handle.name))
            .into_global_arena_ptr();
        if self.structs.contains_key(&name) || self.enums.contains_key(&name) {
            return Ok(());
        }

        // Recurse into local-struct/enum dependencies referenced by direct
        // field types. Move's bytecode verifier rejects cycles, so this
        // terminates without an explicit guard.
        let dependent = self.collect_local_dependencies(def_idx);
        for dep_idx in dependent {
            self.ensure_def(dep_idx)?;
        }

        let bare_ty = self.module.interned_nominal_type_at(struct_handle);
        let def = &self.module.struct_defs[def_idx];
        match &def.field_information {
            StructFieldInformation::Native => bail!("Native fields are deprecated"),
            StructFieldInformation::Declared(_) => {
                let def_index = StructDefinitionIndex(def_idx as u16);
                let field_types = self
                    .module
                    .interned_struct_field_types_at(def_index)
                    .expect("Must be a struct");
                let mut fields = Vec::with_capacity(field_types.len());
                let mut offset = 0u32;
                let mut align = 1u32;
                for &fty in field_types {
                    let (sz, al) = view_type(fty).size_and_align().ok_or_else(|| {
                        anyhow!("Size and alignment is set for non-generic types")
                    })?;
                    offset = align_up(offset, al);
                    align = align.max(al);
                    fields.push(FieldLayout::new(offset, fty));
                    offset += sz;
                }
                let size = align_up(offset, align);
                self.guard
                    .set_nominal_layout(bare_ty, size, align, Some(&fields))?;
                self.structs.insert(name, StructType::new(bare_ty));
            },
            StructFieldInformation::DeclaredVariants(variant_defs) => {
                let def_index = StructDefinitionIndex(def_idx as u16);
                let mut variants = Vec::with_capacity(variant_defs.len());
                for v_idx in 0..variant_defs.len() {
                    let vfields = self
                        .module
                        .interned_variant_field_types_at(def_index, v_idx as u16)
                        .expect("Must be an enum");
                    let fields_slice = self.arena.alloc_slice_copy(vfields);
                    variants.push(VariantFields::new(fields_slice));
                }
                let variants_slice = self.arena.alloc_slice_copy(&variants);
                // Enum size/align is fixed today (heap pointer); the layout
                // slot stores no per-field offsets.
                self.guard.set_nominal_layout(bare_ty, 8, 8, None)?;
                self.enums
                    .insert(name, EnumType::new(bare_ty, variants_slice));
            },
        }
        Ok(())
    }

    /// Walks a def's field types (and variant field types for enums) via
    /// the type pool, collecting `StructDefinitionIndex` ordinals for any
    /// directly-referenced local struct or enum. Composite wrappers
    /// (`Vector`, references) have fixed sizes and don't propagate
    /// dependencies — only direct `Type::Nominal` field types of this
    /// module count.
    fn collect_local_dependencies(&self, def_idx: usize) -> Vec<usize> {
        let def = &self.module.struct_defs[def_idx];
        let def_index = StructDefinitionIndex(def_idx as u16);
        let mut deps = Vec::new();
        match &def.field_information {
            StructFieldInformation::Declared(_) => {
                for &fty in self
                    .module
                    .interned_struct_field_types_at(def_index)
                    .expect("Must be a struct")
                {
                    if let Some(d) = self.local_def_idx_for_type(fty) {
                        deps.push(d);
                    }
                }
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                for v_idx in 0..variants.len() {
                    for &fty in self
                        .module
                        .interned_variant_field_types_at(def_index, v_idx as u16)
                        .expect("Must be an enum")
                    {
                        if let Some(d) = self.local_def_idx_for_type(fty) {
                            deps.push(d);
                        }
                    }
                }
            },
            StructFieldInformation::Native => {},
        }
        deps
    }

    /// Returns the local `StructDefinitionIndex` ordinal for `ty` if it's
    /// a `Type::Nominal` defined in this module, else `None`.
    fn local_def_idx_for_type(&self, ty: InternedType) -> Option<usize> {
        let (executable_id, name) = match view_type(ty) {
            Type::Nominal {
                executable_id,
                name,
                ..
            } => (executable_id, name),
            _ => return None,
        };
        if *executable_id != self.id {
            return None;
        }
        self.local_def_by_name.get(name).copied()
    }
}
