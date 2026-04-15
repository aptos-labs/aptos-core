// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Types for executables (compiled modules / scripts).

use crate::{
    context::types::{
        struct_info_at, try_as_primitive_type, Alignment, FieldLayout, FieldOffset, StructLayout,
        Type, EMPTY_LIST,
    },
    ArenaRef, ExecutionGuard,
};
use anyhow::{anyhow, bail};
use fxhash::FxBuildHasher;
use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{ExecutableId, FrameLayoutInfo, Function, SortedSafePointEntries};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructDefinition, StructFieldInformation, StructHandleIndex},
    CompiledModule,
};
use parking_lot::Mutex;
use std::collections::HashMap;

pub struct StructType {
    /// Struct type signature. Invariant: stored type is always
    /// [`Type::Struct`].
    ty: GlobalArenaPtr<Type>,
}

pub struct EnumType {
    /// Enum type signature. Invariant: stored type is always
    /// [`Type::Enum`].
    ty: GlobalArenaPtr<Type>,
    /// Per-variant field types, indexed by variant tag.
    #[allow(dead_code)]
    variants: ExecutableArenaPtr<[VariantFields]>,
}

/// Field types for a single enum variant.
#[derive(Copy, Clone)]
pub struct VariantFields {
    #[allow(dead_code)]
    fields: ExecutableArenaPtr<[GlobalArenaPtr<Type>]>,
}

/// A loaded executable (from module or script).
pub struct Executable {
    data: ExecutableData,

    /// Arena where data is allocated for this executable. **Must** be the
    /// last field so that it is dropped after any data structure that holds
    /// pointers into it.
    #[allow(dead_code)]
    arena: Mutex<ExecutableArena>,
}

struct ExecutableData {
    /// Executable ID which uniquely identifies this executable.
    id: GlobalArenaPtr<ExecutableId>,
    /// Non-generic struct definitions. Invariant: stored type is always
    /// [`Type::Struct`].
    structs: HashMap<GlobalArenaPtr<str>, StructType, FxBuildHasher>,
    /// Non-generic enum definitions.
    enums: HashMap<GlobalArenaPtr<str>, EnumType, FxBuildHasher>,
    /// Non-generic functions.
    functions: HashMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>, FxBuildHasher>,
}

impl Executable {
    /// Returns a non-generic function from this executable. Returns [`None`]
    /// if such function does not exist.
    pub fn get_function(&self, name: ArenaRef<'_, str>) -> Option<&Function> {
        self.data
            .functions
            .get(&name.into_global_arena_ptr())
            .map(|ptr| {
                // SAFETY: Because executable is alive, all its allocations are
                // still valid.
                unsafe { ptr.as_ref_unchecked() }
            })
    }

    /// Returns a non-generic struct from this executable. Returns [`None`]
    /// if such struct does not exist.
    pub fn get_struct(&self, name: ArenaRef<'_, str>) -> Option<&Type> {
        self.data
            .structs
            .get(&name.into_global_arena_ptr())
            .map(|ptr| {
                // SAFETY: Types must be still valid
                unsafe { ptr.ty.as_ref_unchecked() }
            })
    }
}

// TODO: this is likely to change. Placeholder.
// TODO: refactor to own CompiledModule instead of borrowing it (needed for ModuleIR cache).
// Split mutable state into a separate struct to avoid borrow conflicts with self.module.
#[allow(dead_code)]
pub struct ExecutableBuilder<'a, 'guard, 'ctx> {
    // TODO: support scripts.
    module: &'a CompiledModule,
    /// Maps struct handle indices to struct definition indices.
    struct_def_idx: HashMap<StructHandleIndex, usize>,

    /// Stores data for executable that is being built.
    data: ExecutableData,
    /// Stores all allocations for this executable.
    arena: ExecutableArena,
    /// Context for interning.
    guard: &'guard ExecutionGuard<'ctx>,
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Returns a new builder to transform [`CompiledModule`] into an
    /// [`Executable`].
    pub fn executable_builder_for_module<'a, 'guard>(
        &'guard self,
        module: &'a CompiledModule,
    ) -> ExecutableBuilder<'a, 'guard, 'ctx>
    where
        'ctx: 'guard,
    {
        let struct_def_idx = module
            .struct_defs()
            .iter()
            .enumerate()
            .map(|(i, def)| (def.struct_handle, i))
            .collect::<HashMap<_, _>>();

        let id = self.intern_address_name_internal(*module.self_addr(), module.self_name());
        let data = ExecutableData {
            id,
            structs: HashMap::with_hasher(FxBuildHasher::default()),
            enums: HashMap::with_hasher(FxBuildHasher::default()),
            functions: HashMap::with_hasher(FxBuildHasher::default()),
        };

        ExecutableBuilder {
            module,
            struct_def_idx,
            data,
            arena: ExecutableArena::new(),
            guard: self,
        }
    }
}

impl<'a, 'guard, 'ctx> ExecutableBuilder<'a, 'guard, 'ctx> {
    /// Builds an executable from the provided compiled module.
    pub fn build(mut self) -> anyhow::Result<Box<Executable>> {
        // Process struct definitions first (type layout is needed by lowering
        // functions).
        for struct_def in &self.module.struct_defs {
            self.resolve_struct_def(struct_def)?;
        }

        let lowered = specializer::destack_and_lower_module(self.module.clone())?;

        // Indexed by definition index. Generic functions that are not
        // lowered leave their slot as None.
        let mut func_ptrs = vec![None; lowered.functions.len()];
        for (def_idx, lowered_fn) in lowered.functions.into_iter().enumerate() {
            if let Some(lf) = lowered_fn {
                let name = self
                    .guard
                    .intern_identifier_internal(self.module.identifier_at(lf.name_idx));
                let code = self.arena.alloc_slice_fill_iter(lf.code);
                let func = Function {
                    name,
                    code,
                    args_size: lf.args_size,
                    args_and_locals_size: lf.args_and_locals_size,
                    extended_frame_size: lf.extended_frame_size,
                    // TODO: hardcoded for now.
                    zero_frame: false,
                    frame_layout: FrameLayoutInfo::empty(&self.arena),
                    safe_point_layouts: SortedSafePointEntries::empty(&self.arena),
                };
                let ptr = self.arena.alloc(func);
                self.data.functions.insert(name, ptr);
                func_ptrs[def_idx] = Some(ptr);
            }
        }

        // Patch CallFunc to CallLocalFunc using definition-indexed func_ptrs.
        // SAFETY: We have exclusive access — the executable is being built
        // and no concurrent readers exist. The arena outlives the executable.
        unsafe { Function::resolve_calls(&func_ptrs) };

        Ok(Box::new(Executable {
            data: self.data,
            arena: Mutex::new(self.arena),
        }))
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a, 'guard, 'ctx> ExecutableBuilder<'a, 'guard, 'ctx> {
    /// Resolves a struct or enum definition.
    ///
    /// For structs, computes layouts  **eagerly** by interning each field type
    /// recursively and computing offsets inline. For now, this implements
    /// C-style struct layout. For enums, only variant field types are interned
    /// (enum type-level size is always fixed).
    fn resolve_struct_def(
        &mut self,
        struct_def: &StructDefinition,
    ) -> anyhow::Result<GlobalArenaPtr<Type>> {
        let handle = self.module.struct_handle_at(struct_def.struct_handle);
        if !handle.type_parameters.is_empty() {
            todo!("Generic structs / enums not yet supported");
        }

        let name = self
            .guard
            .intern_identifier_internal(self.module.identifier_at(handle.name));
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
                if let Some(ptr) = self.data.structs.get(&name) {
                    return Ok(ptr.ty);
                }

                // If not yet processed, it is possible that struct type is
                // cached in global arena (because it is not changing under
                // upgrades).
                let tok = SignatureToken::Struct(struct_def.struct_handle);
                if let Some(ptr) = self
                    .guard
                    .get_interned_type_pointer_internal(&tok, self.module)
                {
                    self.data.structs.insert(name, StructType { ty: ptr });
                    return Ok(ptr);
                }

                // Intern each field type and compute layout metadata inline.
                let mut fields = Vec::with_capacity(field_defs.len());
                let mut offset = 0;
                let mut align = 1;

                for field in field_defs {
                    let field_ty = self.intern_signature_token(&field.signature.0)?;

                    // SAFETY: Type was just interned, pointer is valid (arena
                    // has not been reset during execution phase).
                    let field_ty_ref = unsafe { field_ty.as_ref_unchecked() };

                    let (field_size, field_align) =
                        field_ty_ref.size_and_align().ok_or_else(|| {
                            anyhow!("Size and alignment is set for non-generic types")
                        })?;
                    offset = align_up(offset, field_align);
                    align = align.max(field_align);

                    fields.push(FieldLayout::new(offset, field_ty));
                    offset += field_size;
                }

                let size = align_up(offset, align);
                // Note: fields are not interned, but they are also not used
                // for equality or hash - but simply cache layouts in-place.
                let fields = self.guard.global_arena.alloc_slice_copy(&fields);
                let layout = StructLayout::new(size, align, fields);

                let ty = self.guard.global_arena.alloc(Type::Struct {
                    executable_id: self.data.id,
                    name,
                    ty_args: GlobalArenaPtr::from_static(&EMPTY_LIST),
                    layout: Some(layout),
                });
                let ptr = self.guard.insert_allocated_type_pointer_internal(ty);

                self.data.structs.insert(name, StructType { ty: ptr });
                Ok(ptr)
            },
            StructFieldInformation::DeclaredVariants(variant_defs) => {
                if let Some(enum_def) = self.data.enums.get(&name) {
                    return Ok(enum_def.ty);
                }

                // If not yet processed, it is possible that enum type is
                // cached in global arena/
                let tok = SignatureToken::Struct(struct_def.struct_handle);
                let ty = match self
                    .guard
                    .get_interned_type_pointer_internal(&tok, self.module)
                {
                    Some(ptr) => ptr,
                    None => {
                        let ty = self.guard.global_arena.alloc(Type::Enum {
                            executable_id: self.data.id,
                            name,
                            ty_args: GlobalArenaPtr::from_static(&EMPTY_LIST),
                        });
                        self.guard.insert_allocated_type_pointer_internal(ty)
                    },
                };

                let mut variants = Vec::with_capacity(variant_defs.len());
                for variant_def in variant_defs {
                    let mut fields = Vec::with_capacity(variant_def.fields.len());
                    for field in &variant_def.fields {
                        let field_ty = self.intern_signature_token(&field.signature.0)?;
                        fields.push(field_ty);
                    }
                    let fields = self.arena.alloc_slice_copy(&fields);
                    variants.push(VariantFields { fields });
                }
                let variants = self.arena.alloc_slice_copy(&variants);
                self.data.enums.insert(name, EnumType { ty, variants });
                Ok(ty)
            },
        }
    }

    /// Interns a signature token as a [`Type`]. Primitives return static
    /// pointers. Composite types are arena-allocated and deduplicated. Because
    /// allocation is outside the lock, it is possible that some allocations
    /// will not be used. This is a design choice, as the memory waste is
    /// bounded by available concurrency and the number of unique types.
    fn intern_signature_token(
        &mut self,
        token: &SignatureToken,
        // TODO:
        //   In the future, we need to pass type arguments so we can resolve
        //   field layouts of fully-instantiated generics.
    ) -> anyhow::Result<GlobalArenaPtr<Type>> {
        // Primitives are static allocations - fast path.
        if let Some(ptr) = try_as_primitive_type(token) {
            return Ok(ptr);
        }

        // Special case for structs / enums because we need to resolve local
        // and non-local definitions.
        if let Some((idx, _ty_args)) = token.struct_idx_and_ty_args() {
            return match self.struct_def_idx.get(idx).copied() {
                // TODO: handle type arguments for generic structs!
                Some(idx) => self.resolve_struct_def(&self.module.struct_defs[idx]),
                None => {
                    // TODO:
                    //   If this type is a struct or an enum that is non-local,
                    //   assume it must be interned & resolved. In the future,
                    //   this case need to load executable dependency first.
                    self.guard
                        .get_interned_type_pointer_internal(token, self.module)
                        .ok_or_else(|| {
                            let (address, module_name, struct_name) = struct_info_at(self.module, *idx);
                            anyhow!(
                                "Non-local type not yet interned (transitive dependency not loaded): {}::{}::{}",
                                address,
                                module_name,
                                struct_name
                            )
                        })
                },
            };
        }

        // Otherwise, this is a composite type. Check if it has been interned
        // before first.
        if let Some(ptr) = self
            .guard
            .get_interned_type_pointer_internal(token, self.module)
        {
            return Ok(ptr);
        }

        // TODO: non-recursive implementation.

        // Cache miss: need to allocate and intern. Allocation is outside the
        // lock to avoid contention and may recurse into inner type interning.
        let ptr = match token {
            SignatureToken::Reference(inner) => {
                let inner = self.intern_signature_token(inner.as_ref())?;
                self.guard.global_arena.alloc(Type::ImmutRef { inner })
            },
            SignatureToken::MutableReference(inner) => {
                let inner = self.intern_signature_token(inner.as_ref())?;
                self.guard.global_arena.alloc(Type::MutRef { inner })
            },
            SignatureToken::Vector(elem_token) => {
                let elem = self.intern_signature_token(elem_token.as_ref())?;
                self.guard.global_arena.alloc(Type::Vector { elem })
            },
            SignatureToken::Function(arg_tokens, result_tokens, abilities) => {
                let args = self.intern_signature_token_list(arg_tokens)?;
                let results = self.intern_signature_token_list(result_tokens)?;
                self.guard.global_arena.alloc(Type::Function {
                    args,
                    results,
                    abilities: *abilities,
                })
            },
            SignatureToken::TypeParameter(idx) => {
                self.guard.global_arena.alloc(Type::TypeParam { idx: *idx })
            },

            // Primitives handled above.
            SignatureToken::Bool
            | SignatureToken::U8
            | SignatureToken::U16
            | SignatureToken::U32
            | SignatureToken::U64
            | SignatureToken::U128
            | SignatureToken::U256
            | SignatureToken::I8
            | SignatureToken::I16
            | SignatureToken::I32
            | SignatureToken::I64
            | SignatureToken::I128
            | SignatureToken::I256
            | SignatureToken::Address
            | SignatureToken::Signer
            | SignatureToken::Struct(..)
            | SignatureToken::StructInstantiation(..) => bail!("Must be already handled"),
        };

        // Insert and deduplicate the pointer, to ensure uniqueness even if
        // there are race conditions.
        Ok(self.guard.insert_allocated_type_pointer_internal(ptr))
    }

    /// Interns a list of signature tokens as a canonical type list. Empty
    /// lists return a static pointer (no allocations). Non-empty lists are
    /// arena-allocated and deduplicated. Because allocation is outside the
    /// lock, it is possible that some allocations will not be used. This is
    /// a design choice - the memory waste is bounded by available concurrency
    /// and the number of unique type lists.
    fn intern_signature_token_list(
        &mut self,
        tokens: &[SignatureToken],
    ) -> anyhow::Result<GlobalArenaPtr<[GlobalArenaPtr<Type>]>> {
        if tokens.is_empty() {
            return Ok(GlobalArenaPtr::from_static(&EMPTY_LIST));
        }

        if let Some(ptr) = self
            .guard
            .get_interned_type_list_internal(tokens, self.module)
        {
            return Ok(ptr);
        }

        let mut types = Vec::with_capacity(tokens.len());
        for token in tokens {
            types.push(self.intern_signature_token(token)?);
        }
        let ptr = self.guard.global_arena.alloc_slice_copy(&types);
        Ok(self.guard.insert_allocated_type_list_internal(ptr))
    }
}

/// Rounds the value up to the next multiple of alignment.
///
/// **Pre-condition:** Align is non-zero and is a power of two.
fn align_up(offset: FieldOffset, align: Alignment) -> Alignment {
    debug_assert!(align > 0 && align.is_power_of_two());
    (offset + align - 1) & !(align - 1)
}
