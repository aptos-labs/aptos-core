// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Builds an [`Executable`] from a [`CompiledModule`] by resolving its
//! struct/enum types and accumulating lowered functions.
//!
//! Type interning primitives live in `mono-move-global-context`; this module
//! orchestrates the module-level walk (struct defs, enum defs, layout
//! computation) and the final assembly into an `Executable`.

use anyhow::{anyhow, bail};
use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    types::{align_up, EMPTY_TYPE_LIST},
    EnumType, Executable, ExecutableId, FrameLayoutInfo, Function, SortedSafePointEntries,
    StructType, VariantFields,
};
use mono_move_global_context::{
    struct_info_at, walk_sig_token, ExecutionGuard, FieldLayout, InternedType, StructResolver,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        IdentifierIndex, SignatureToken, StructDefinition, StructFieldInformation,
        StructHandleIndex,
    },
    CompiledModule,
};
use shared_dsa::UnorderedMap;

// TODO: this is likely to change. Placeholder.
// TODO: refactor to own CompiledModule instead of borrowing it (needed for ModuleIR cache).
// Split mutable state into a separate struct to avoid borrow conflicts with self.module.
#[allow(dead_code)]
pub struct ExecutableBuilder<'a, 'guard, 'ctx> {
    // TODO: support scripts.
    module: &'a CompiledModule,
    /// Maps struct handle indices to struct definition indices.
    struct_def_idx: UnorderedMap<StructHandleIndex, usize>,

    /// Executable ID.
    id: GlobalArenaPtr<ExecutableId>,
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
    /// Next `FunctionDefinitionIndex` to fill. Advances on every
    /// `add_function` / `skip_function` call so we can map def → handle via
    /// `module.function_defs[..]`.
    next_def_idx: usize,
    /// Stores all allocations for this executable.
    arena: ExecutableArena,
    /// Context for interning.
    guard: &'guard ExecutionGuard<'ctx>,
}

impl<'a, 'guard, 'ctx> ExecutableBuilder<'a, 'guard, 'ctx> {
    /// Creates a new builder for transforming `module` into an [`Executable`].
    pub fn new(guard: &'guard ExecutionGuard<'ctx>, module: &'a CompiledModule) -> Self
    where
        'ctx: 'guard,
    {
        let struct_def_idx = module
            .struct_defs()
            .iter()
            .enumerate()
            .map(|(i, def)| (def.struct_handle, i))
            .collect::<UnorderedMap<_, _>>();

        let id = guard
            .intern_address_name(module.self_addr(), module.self_name())
            .into_global_arena_ptr();

        ExecutableBuilder {
            module,
            struct_def_idx,
            id,
            structs: UnorderedMap::new(),
            enums: UnorderedMap::new(),
            functions: UnorderedMap::new(),
            func_ptrs: vec![None; module.function_handles.len()],
            next_def_idx: 0,
            arena: ExecutableArena::new(),
            guard,
        }
    }

    /// Resolve all struct and enum type definitions in the module.
    /// Must be called before `finish()`.
    pub fn resolve_types(&mut self) -> anyhow::Result<()> {
        for struct_def in &self.module.struct_defs {
            self.resolve_struct_def(struct_def)?;
        }
        Ok(())
    }

    /// Returns the module being built.
    pub fn module(&self) -> &CompiledModule {
        self.module
    }

    /// Returns a struct type table mapping `StructHandleIndex` ordinals to
    /// interned type pointers. Call after `resolve_types()`.
    ///
    /// For struct handles that were resolved (non-generic, local), the table
    /// contains their interned type pointer. For unresolved handles
    /// (generic, non-local), the entry may be missing or use a placeholder.
    pub fn struct_type_table(&self) -> Vec<InternedType> {
        let num_handles = self.module.struct_handles.len();
        let mut table = vec![mono_move_core::types::BOOL_TY; num_handles];
        for struct_def in &self.module.struct_defs {
            let handle = self.module.struct_handle_at(struct_def.struct_handle);
            let name = self
                .guard
                .intern_identifier(self.module.identifier_at(handle.name))
                .into_global_arena_ptr();
            if let Some(st) = self.structs.get(&name) {
                table[struct_def.struct_handle.0 as usize] = st.ty();
            } else if let Some(et) = self.enums.get(&name) {
                table[struct_def.struct_handle.0 as usize] = et.ty();
            }
        }
        table
    }

    /// Adds a lowered function to the executable being built.
    /// Returns the definition index for call patching.
    pub fn add_function(
        &mut self,
        name_idx: IdentifierIndex,
        code: Vec<mono_move_core::MicroOp>,
        args_size: usize,
        args_and_locals_size: usize,
        extended_frame_size: usize,
    ) -> usize {
        let name = self
            .guard
            .intern_identifier(self.module.identifier_at(name_idx))
            .into_global_arena_ptr();
        let code = self.arena.alloc_slice_fill_iter(code);
        let func = Function {
            name,
            code,
            args_size,
            args_and_locals_size,
            extended_frame_size,
            // TODO: hardcoded for now.
            zero_frame: false,
            frame_layout: FrameLayoutInfo::empty(&self.arena),
            safe_point_layouts: SortedSafePointEntries::empty(&self.arena),
        };
        let ptr = self.arena.alloc(func);
        self.functions.insert(name, ptr);
        let def_idx = self.next_def_idx;
        let handle_idx = self.module.function_defs[def_idx].function.0 as usize;
        self.func_ptrs[handle_idx] = Some(ptr);
        self.next_def_idx += 1;
        def_idx
    }

    /// Records a definition slot with no lowered function (e.g., generic or
    /// native function).
    pub fn skip_function(&mut self) {
        self.next_def_idx += 1;
    }

    /// Finishes building the executable. Call after all functions have been
    /// added via `add_function` / `skip_function`.
    pub fn finish(self) -> anyhow::Result<Box<Executable>> {
        // Patch CallFunc to CallLocalFunc using definition-indexed func_ptrs.
        // SAFETY: We have exclusive access — the executable is being built
        // and no concurrent readers exist. The arena outlives the executable.
        unsafe { Function::resolve_calls(&self.func_ptrs) };

        Ok(Executable::new(
            self.id,
            self.structs,
            self.enums,
            self.functions,
            self.arena,
        ))
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a, 'guard, 'ctx> ExecutableBuilder<'a, 'guard, 'ctx> {
    /// Resolves a struct or enum definition.
    ///
    /// For structs, computes layouts **eagerly** by interning each field type
    /// and computing offsets inline. For now, this implements C-style struct
    /// layout. For enums, only variant field types are interned (enum
    /// type-level size is always fixed).
    fn resolve_struct_def(
        &mut self,
        struct_def: &StructDefinition,
    ) -> anyhow::Result<InternedType> {
        let handle = self.module.struct_handle_at(struct_def.struct_handle);
        if !handle.type_parameters.is_empty() {
            todo!("Generic structs / enums not yet supported");
        }

        let name = self
            .guard
            .intern_identifier(self.module.identifier_at(handle.name))
            .into_global_arena_ptr();
        match &struct_def.field_information {
            StructFieldInformation::Native => bail!("Native fields are deprecated"),
            StructFieldInformation::Declared(field_defs) => {
                // Check if already visited. For example, if we have structs:
                //
                // struct A { x: u64 }
                // struct B { x: A }
                //
                // we do not need to recompute A's type information and can use
                // cached data.
                if let Some(st) = self.structs.get(&name) {
                    return Ok(st.ty());
                }

                // If not yet processed, the struct type may already be cached
                // in the global arena (because it is not changing under
                // upgrades).
                let tok = SignatureToken::Struct(struct_def.struct_handle);
                if let Some(ptr) = self.guard.try_intern_for_module(&tok, self.module) {
                    self.structs.insert(name, StructType::new(ptr));
                    return Ok(ptr);
                }

                // Intern each field type and compute layout metadata inline.
                let mut fields = Vec::with_capacity(field_defs.len());
                let mut offset = 0;
                let mut align = 1;

                for field in field_defs {
                    let field_ty = self.intern_signature_token(&field.signature.0)?;

                    let (field_size, field_align) = self
                        .guard
                        .type_data(field_ty)
                        .size_and_align()
                        .ok_or_else(|| {
                            anyhow!("Size and alignment is set for non-generic types")
                        })?;
                    offset = align_up(offset, field_align);
                    align = align.max(field_align);

                    fields.push(FieldLayout::new(offset, field_ty));
                    offset += field_size;
                }

                let size = align_up(offset, align);
                let ptr = self.guard.intern_struct_type(
                    self.id,
                    name,
                    EMPTY_TYPE_LIST,
                    size,
                    align,
                    &fields,
                );

                self.structs.insert(name, StructType::new(ptr));
                Ok(ptr)
            },
            StructFieldInformation::DeclaredVariants(variant_defs) => {
                if let Some(enum_def) = self.enums.get(&name) {
                    return Ok(enum_def.ty());
                }

                // If not yet processed, the enum type may already be cached
                // in the global arena.
                let tok = SignatureToken::Struct(struct_def.struct_handle);
                let ty = self
                    .guard
                    .try_intern_for_module(&tok, self.module)
                    .unwrap_or_else(|| self.guard.intern_enum_type(self.id, name, EMPTY_TYPE_LIST));

                let mut variants = Vec::with_capacity(variant_defs.len());
                for variant_def in variant_defs {
                    let mut fields = Vec::with_capacity(variant_def.fields.len());
                    for field in &variant_def.fields {
                        let field_ty = self.intern_signature_token(&field.signature.0)?;
                        fields.push(field_ty);
                    }
                    let fields = self.arena.alloc_slice_copy(&fields);
                    variants.push(VariantFields::new(fields));
                }
                let variants = self.arena.alloc_slice_copy(&variants);
                self.enums.insert(name, EnumType::new(ty, variants));
                Ok(ty)
            },
        }
    }

    /// Interns a signature token as a [`Type`], delegating composite variants
    /// to [`walk_sig_token`]. This wrapper keeps the per-token cache fast
    /// path (avoids re-walking tokens already seen during this module's
    /// resolution).
    fn intern_signature_token(
        &mut self,
        token: &SignatureToken,
        // TODO:
        //   In the future, we need to pass type arguments so we can resolve
        //   field layouts of fully-instantiated generics.
    ) -> anyhow::Result<InternedType> {
        if let Some(ptr) = self.guard.try_intern_for_module(token, self.module) {
            return Ok(ptr);
        }
        walk_sig_token(token, self.guard, self)
    }
}

impl<'a, 'guard, 'ctx> StructResolver for ExecutableBuilder<'a, 'guard, 'ctx> {
    fn resolve_struct(
        &mut self,
        struct_handle: StructHandleIndex,
        ty_args: &[SignatureToken],
    ) -> anyhow::Result<InternedType> {
        // TODO: handle type arguments for generic structs!
        match self.struct_def_idx.get(&struct_handle).copied() {
            Some(def_idx) => self.resolve_struct_def(&self.module.struct_defs[def_idx]),
            None => {
                // TODO:
                //   If this type is a struct or an enum that is non-local,
                //   assume it must be interned & resolved. In the future,
                //   this case need to load executable dependency first.
                let token = if ty_args.is_empty() {
                    SignatureToken::Struct(struct_handle)
                } else {
                    SignatureToken::StructInstantiation(struct_handle, ty_args.to_vec())
                };
                self.guard
                    .try_intern_for_module(&token, self.module)
                    .ok_or_else(|| {
                        let (address, module_name, struct_name) =
                            struct_info_at(self.module, struct_handle);
                        anyhow!(
                            "Non-local type not yet interned (transitive dependency not loaded): {}::{}::{}",
                            address,
                            module_name,
                            struct_name
                        )
                    })
            },
        }
    }
}
