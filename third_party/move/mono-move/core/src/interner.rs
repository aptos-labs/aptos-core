// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Interning APIs.
//!
//! Today this module exposes the interning leaves needed to canonicalize
//! composite [`Type`](crate::types::Type) nodes into the global type arena.
//! Other interning concerns — interning of address/name pairs into
//! [`ExecutableId`](crate::executable::ExecutableId)s and of identifier
//! strings are expected to migrate here as the abstract interning interface
//! grows.

use crate::types::{InternedType, InternedTypeList};
use move_binary_format::file_format::{SignatureToken, StructHandleIndex};
use move_core_types::ability::AbilitySet;

/// Intern composite [`Type`](crate::types::Type) leaves into the global type
/// arena. Implementations deduplicate so that pointer equality implies
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
}

/// Resolves a struct handle (with its type arguments) to an interned type
/// pointer.
pub trait StructResolver {
    fn resolve_struct(
        &mut self,
        struct_handle: StructHandleIndex,
        ty_args: &[SignatureToken],
    ) -> anyhow::Result<InternedType>;
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
pub fn walk_sig_token<I: Interner, R: StructResolver>(
    token: &SignatureToken,
    interner: &I,
    resolver: &mut R,
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
            let elem = walk_sig_token(inner, interner, resolver)?;
            interner.vector_of(elem)
        },
        SignatureToken::Reference(inner) => {
            let inner = walk_sig_token(inner, interner, resolver)?;
            interner.immut_ref_of(inner)
        },
        SignatureToken::MutableReference(inner) => {
            let inner = walk_sig_token(inner, interner, resolver)?;
            interner.mut_ref_of(inner)
        },
        SignatureToken::Function(args, results, abilities) => {
            let arg_ptrs = args
                .iter()
                .map(|t| walk_sig_token(t, interner, resolver))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let result_ptrs = results
                .iter()
                .map(|t| walk_sig_token(t, interner, resolver))
                .collect::<anyhow::Result<Vec<_>>>()?;
            let args = interner.type_list_of(&arg_ptrs);
            let results = interner.type_list_of(&result_ptrs);
            interner.function_of(args, results, *abilities)
        },
        SignatureToken::Struct(sh_idx) => resolver.resolve_struct(*sh_idx, &[])?,
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            resolver.resolve_struct(*sh_idx, tys)?
        },
    })
}
