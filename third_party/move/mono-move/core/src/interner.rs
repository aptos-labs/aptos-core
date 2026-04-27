// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interning APIs.

use crate::{
    types::{InternedType, InternedTypeList},
    ExecutableId,
};
use mono_move_alloc::GlobalArenaPtr;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructHandle, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{ability::AbilitySet, account_address::AccountAddress, identifier::IdentStr};

/// Pointer to interned Move identifier allocated in global arena.
pub type InternedIdentifier = GlobalArenaPtr<str>;

/// Pointer to interned module ID allocated in global arena.
pub type InternedModuleId = GlobalArenaPtr<ExecutableId>;

/// Interns Move file format types into efficient pointer-based implementation
/// where data is allocated in arena.
///
/// # Invariant
///
/// Implementations deduplicate allocations, so that pointer equality implies
/// structural equality.
pub trait Interner {
    /// Returns a type parameter with the specified index. Note that pointer
    /// equality of any two interned type parameters is structural only. Two
    /// parameters with index 0 but at different scope may represent different
    /// types (but intern to the same pointer).
    fn type_param_of(&self, idx: u16) -> InternedType;

    /// Returns a vector of the specified type.
    fn vector_of(&self, elem: InternedType) -> InternedType;

    /// Returns an immutable reference to the specified type.
    fn immut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a mutable reference to the specified type.
    fn mut_ref_of(&self, inner: InternedType) -> InternedType;

    /// Returns a function type with the given argument and result type lists
    /// and ability set.
    fn function_of(
        &self,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    ) -> InternedType;

    /// Returns an interned list of types.
    fn type_list_of(&self, types: &[InternedType]) -> InternedTypeList;

    /// Returns the interned nominal (struct or enum) identity.
    fn nominal_of(
        &self,
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> InternedType;

    /// Returns the interned IR corresponding to (address, module name) pair
    /// that identifies a module.
    fn module_id_of(&self, address: &AccountAddress, name: &IdentStr) -> InternedModuleId;

    /// Returns an interned string identifier.
    fn identifier_of(&self, identifier: &IdentStr) -> InternedIdentifier;
}

/// Recursively interns `token` into the global type arena. Composite leaves
/// go through `interner`; struct/enum tokens delegate to `resolver`.
///
/// TODO: non-recursive implementation. Coordinate with the similar TODO on
/// `TypeInternerKey`'s `Hash` impl in `types.rs`.
///
/// TODO (perf): probe-before-allocate for composite tokens.
///
/// Right now, every composite variant (Vector, Reference, MutableReference,
/// Function, and the StructInstantiation path through the resolver) allocates a
/// fresh `Type` node in the arena and then hands it to the interner, which
/// discards the new allocation whenever an equivalent entry already exists. For
/// modules with shared signatures (common: many handles reference the same
/// `SignatureIndex`, and `vector<T>` / `&T` appear repeatedly), this means the
/// fast path pays one arena allocation + a dedup probe per occurrence instead
/// of a single probe.
pub fn intern_sig_token(
    token: &SignatureToken,
    module: &CompiledModule,
    interner: &impl Interner,
) -> anyhow::Result<InternedType> {
    use crate::types as ty;
    Ok(match token {
        SignatureToken::Bool => ty::BOOL_TY,
        SignatureToken::U8 => ty::U8_TY,
        SignatureToken::U16 => ty::U16_TY,
        SignatureToken::U32 => ty::U32_TY,
        SignatureToken::U64 => ty::U64_TY,
        SignatureToken::U128 => ty::U128_TY,
        SignatureToken::U256 => ty::U256_TY,
        SignatureToken::I8 => ty::I8_TY,
        SignatureToken::I16 => ty::I16_TY,
        SignatureToken::I32 => ty::I32_TY,
        SignatureToken::I64 => ty::I64_TY,
        SignatureToken::I128 => ty::I128_TY,
        SignatureToken::I256 => ty::I256_TY,
        SignatureToken::Address => ty::ADDRESS_TY,
        SignatureToken::Signer => ty::SIGNER_TY,
        SignatureToken::TypeParameter(idx) => interner.type_param_of(*idx),
        SignatureToken::Vector(inner) => {
            let elem = intern_sig_token(inner, module, interner)?;
            interner.vector_of(elem)
        },
        SignatureToken::Reference(inner) => {
            let inner = intern_sig_token(inner, module, interner)?;
            interner.immut_ref_of(inner)
        },
        SignatureToken::MutableReference(inner) => {
            let inner = intern_sig_token(inner, module, interner)?;
            interner.mut_ref_of(inner)
        },
        SignatureToken::Function(args, results, abilities) => {
            let arg_ptrs = args
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let result_ptrs = results
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let args = interner.type_list_of(&arg_ptrs);
            let results = interner.type_list_of(&result_ptrs);
            interner.function_of(args, results, *abilities)
        },
        SignatureToken::Struct(sh_idx) => {
            let (module_id, struct_name) = intern_struct_info(*sh_idx, module, interner);
            interner.nominal_of(module_id, struct_name, ty::EMPTY_TYPE_LIST)
        },
        SignatureToken::StructInstantiation(sh_idx, ty_args) => {
            let (module_id, struct_name) = intern_struct_info(*sh_idx, module, interner);
            let ty_args = ty_args
                .iter()
                .map(|t| intern_sig_token(t, module, interner))
                .collect::<anyhow::Result<Vec<_>>>()?;
            interner.nominal_of(module_id, struct_name, interner.type_list_of(&ty_args))
        },
    })
}

fn intern_struct_info(
    idx: StructHandleIndex,
    module: &CompiledModule,
    interner: &impl Interner,
) -> (InternedModuleId, InternedIdentifier) {
    let struct_handle = module.struct_handle_at(idx);
    intern_struct_handle(struct_handle, module, interner)
}

/// Returns interned module ID and nominal type name for the given handle.
pub fn intern_struct_handle(
    struct_handle: &StructHandle,
    module: &CompiledModule,
    interner: &impl Interner,
) -> (InternedModuleId, InternedIdentifier) {
    let module_handle = module.module_handle_at(struct_handle.module);
    let address = module.address_identifier_at(module_handle.address);
    let module_name = module.identifier_at(module_handle.name);
    let struct_name = module.identifier_at(struct_handle.name);

    let module_id = interner.module_id_of(address, module_name);
    let struct_name = interner.identifier_of(struct_name);
    (module_id, struct_name)
}
