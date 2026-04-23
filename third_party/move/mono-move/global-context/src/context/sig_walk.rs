// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Walker over [`SignatureToken`] that produces interned [`InternedType`].
//! Struct/enum resolution is pluggable via [`StructResolver`], so different
//! call sites (executable building vs. function body lowering) can supply
//! their own struct-handle lookup while sharing the walk over the rest of
//! the variants.

use crate::ExecutionGuard;
use mono_move_core::types::{self as ty, InternedType};
use move_binary_format::file_format::{SignatureToken, StructHandleIndex};

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
/// go through `guard`'s public intern helpers; struct/enum tokens delegate
/// to `resolver`.
///
/// TODO: non-recursive implementation. Coordinate with the similar TODO on
/// `TypeInternerKey`'s `Hash` impl in `types.rs`.
///
/// TODO (perf): probe-before-allocate for composite tokens.
///
/// Right now, every composite variant (Vector, Reference, MutableReference,
/// Function, and the StructInstantiation path through the resolver) allocates a
/// fresh `Type` node in the arena and then hands it to
/// `insert_allocated_type_pointer_internal`, which discards the new allocation
/// whenever an equivalent entry already exists. For modules with shared
/// signatures (common: many handles reference the same `SignatureIndex`, and
/// `vector<T>` / `&T` appear repeatedly), this means the fast path pays one
/// arena allocation + a dedup probe per occurrence instead of a single probe.
///
/// The dedup map already supports a cheaper key: `SignatureTokenKey` implements
/// `Equivalent<TypeInternerKey>`, so we can look up by `(token, module)`
/// without allocating. To take advantage, thread a `&CompiledModule` through
/// this walker and, for each composite token, call
/// `get_interned_type_pointer_internal` first; only recurse into children and
/// allocate on a miss.
pub fn walk_sig_token<R: StructResolver>(
    token: &SignatureToken,
    guard: &ExecutionGuard<'_>,
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
        SignatureToken::TypeParameter(idx) => guard.intern_type_param(*idx),
        SignatureToken::Vector(inner) => {
            let elem = walk_sig_token(inner, guard, resolver)?;
            guard.intern_vector(elem)
        },
        SignatureToken::Reference(inner) => {
            let inner = walk_sig_token(inner, guard, resolver)?;
            guard.intern_immut_ref(inner)
        },
        SignatureToken::MutableReference(inner) => {
            let inner = walk_sig_token(inner, guard, resolver)?;
            guard.intern_mut_ref(inner)
        },
        SignatureToken::Function(args, results, abilities) => {
            let arg_ptrs: Vec<InternedType> = args
                .iter()
                .map(|t| walk_sig_token(t, guard, resolver))
                .collect::<anyhow::Result<_>>()?;
            let result_ptrs: Vec<InternedType> = results
                .iter()
                .map(|t| walk_sig_token(t, guard, resolver))
                .collect::<anyhow::Result<_>>()?;
            let args = guard.intern_type_list(&arg_ptrs);
            let results = guard.intern_type_list(&result_ptrs);
            guard.intern_function_type(args, results, *abilities)
        },
        SignatureToken::Struct(sh_idx) => resolver.resolve_struct(*sh_idx, &[])?,
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            resolver.resolve_struct(*sh_idx, tys)?
        },
    })
}
