// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-interning APIs.

use crate::types::{self as ty, view_type, InternedType, InternedTypeList, Type};
use move_binary_format::file_format::{SignatureToken, StructHandleIndex};
use move_core_types::ability::AbilitySet;

/// Intern composite [`Type`](crate::types::Type) leaves into the global type
/// arena. Implementations deduplicate so that pointer equality implies
/// structural equality.
///
/// The trait covers exactly the leaves [`walk_sig_token`] and the SSA
/// conversion pass need; struct/enum interning is intentionally not here (it
/// carries layout state that only the executable builder owns).
pub trait Interner {
    fn intern_type_param(&self, idx: u16) -> InternedType;

    fn intern_vec(&self, elem: InternedType) -> InternedType;

    fn intern_immut_ref(&self, inner: InternedType) -> InternedType;

    fn intern_mut_ref(&self, inner: InternedType) -> InternedType;

    fn intern_func(
        &self,
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    ) -> InternedType;

    fn intern_type_list(&self, types: &[InternedType]) -> InternedTypeList;

    /// Converts `&mut T` to `&T` by interning the immutable counterpart.
    /// Errors if `mut_ref` is not a mutable reference type.
    fn convert_mut_to_immut_ref(&self, mut_ref: InternedType) -> anyhow::Result<InternedType> {
        let Type::MutRef { inner } = view_type(mut_ref) else {
            anyhow::bail!("convert_mut_to_immut_ref: expected MutRef");
        };
        Ok(self.intern_immut_ref(*inner))
    }

    /// Strips the reference from `&T` or `&mut T`, returning `T`.
    /// Errors if `ref_ty` is not a reference type.
    fn strip_ref(&self, ref_ty: InternedType) -> anyhow::Result<InternedType> {
        let (Type::ImmutRef { inner } | Type::MutRef { inner }) = view_type(ref_ty) else {
            anyhow::bail!("strip_ref: expected reference type");
        };
        Ok(*inner)
    }
}

/// Resolves a struct handle (with its type arguments) to an interned type
/// pointer. Implementations may pre-build a table (specializer) or resolve
/// on demand (executable builder).
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
        SignatureToken::TypeParameter(idx) => interner.intern_type_param(*idx),
        SignatureToken::Vector(inner) => {
            let elem = walk_sig_token(inner, interner, resolver)?;
            interner.intern_vec(elem)
        },
        SignatureToken::Reference(inner) => {
            let inner = walk_sig_token(inner, interner, resolver)?;
            interner.intern_immut_ref(inner)
        },
        SignatureToken::MutableReference(inner) => {
            let inner = walk_sig_token(inner, interner, resolver)?;
            interner.intern_mut_ref(inner)
        },
        SignatureToken::Function(args, results, abilities) => {
            let arg_ptrs: Vec<InternedType> = args
                .iter()
                .map(|t| walk_sig_token(t, interner, resolver))
                .collect::<anyhow::Result<_>>()?;
            let result_ptrs: Vec<InternedType> = results
                .iter()
                .map(|t| walk_sig_token(t, interner, resolver))
                .collect::<anyhow::Result<_>>()?;
            let args = interner.intern_type_list(&arg_ptrs);
            let results = interner.intern_type_list(&result_ptrs);
            interner.intern_func(args, results, *abilities)
        },
        SignatureToken::Struct(sh_idx) => resolver.resolve_struct(*sh_idx, &[])?,
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            resolver.resolve_struct(*sh_idx, tys)?
        },
    })
}
