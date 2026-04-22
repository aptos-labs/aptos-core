// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion from [`SignatureToken`] to [`InternedType`] via the [`ExecutionGuard`].
//!
//! Delegates to [`mono_move_global_context::walk_sig_token`], supplying a
//! [`StructResolver`] that looks up entries in a pre-built table.

use anyhow::{anyhow, Result};
use mono_move_core::types::InternedType;
use mono_move_global_context::{walk_sig_token, ExecutionGuard, StructResolver};
use move_binary_format::file_format::{SignatureToken, StructHandleIndex};

/// Convert a single [`SignatureToken`] to [`InternedType`].
///
/// `struct_types` maps [`StructHandleIndex`] ordinals to pre-resolved
/// interned type pointers.
pub(crate) fn convert_sig_token(
    tok: &SignatureToken,
    guard: &ExecutionGuard<'_>,
    struct_types: &[InternedType],
) -> Result<InternedType> {
    let mut resolver = TableResolver { struct_types };
    walk_sig_token(tok, guard, &mut resolver)
}

/// Convert a slice of [`SignatureToken`]s to [`Vec`]<[`InternedType`]>.
pub(crate) fn convert_sig_tokens(
    toks: &[SignatureToken],
    guard: &ExecutionGuard<'_>,
    struct_types: &[InternedType],
) -> Result<Vec<InternedType>> {
    toks.iter()
        .map(|t| convert_sig_token(t, guard, struct_types))
        .collect()
}

/// Resolves struct handles via direct table lookup. The table is indexed by
/// [`StructHandleIndex`] ordinal.
struct TableResolver<'a> {
    struct_types: &'a [InternedType],
}

impl StructResolver for TableResolver<'_> {
    fn resolve_struct(
        &mut self,
        struct_handle: StructHandleIndex,
        _ty_args: &[SignatureToken],
    ) -> Result<InternedType> {
        // TODO: proper generic instantiation interning. For now, fall back
        // to the base struct type for both `SignatureToken::Struct` and
        // `SignatureToken::StructInstantiation` — the specializer only needs
        // the base type for polymorphic code.
        self.struct_types
            .get(struct_handle.0 as usize)
            .copied()
            .ok_or_else(|| anyhow!("struct handle index {} out of bounds", struct_handle.0))
    }
}
