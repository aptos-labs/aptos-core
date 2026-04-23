// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Conversion from [`SignatureToken`] to [`InternedType`] via the [`ExecutionGuard`].
//!
//! Delegates to [`mono_move_global_context::walk_sig_token`], supplying a
//! [`StructResolver`] that looks up entries in a pre-built table.

use anyhow::{bail, Result};
use mono_move_core::types::InternedType;
use mono_move_global_context::{walk_sig_token, ExecutionGuard, StructResolver};
use move_binary_format::file_format::{SignatureToken, StructHandleIndex};

/// Convert a single [`SignatureToken`] to [`InternedType`].
///
/// `struct_types` maps [`StructHandleIndex`] ordinals to pre-resolved
/// interned type pointers; unresolved entries are `None`.
pub(crate) fn convert_sig_token(
    tok: &SignatureToken,
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<InternedType> {
    let mut resolver = TableResolver { struct_types };
    walk_sig_token(tok, guard, &mut resolver)
}

/// Convert a slice of [`SignatureToken`]s to [`Vec`]<[`InternedType`]>.
pub(crate) fn convert_sig_tokens(
    toks: &[SignatureToken],
    guard: &ExecutionGuard<'_>,
    struct_types: &[Option<InternedType>],
) -> Result<Vec<InternedType>> {
    toks.iter()
        .map(|t| convert_sig_token(t, guard, struct_types))
        .collect()
}

/// Resolves struct handles via direct table lookup. The table is indexed by
/// [`StructHandleIndex`] ordinal; `None` entries denote handles the
/// orchestrator could not resolve (imported from another module, or
/// generic).
struct TableResolver<'a> {
    struct_types: &'a [Option<InternedType>],
}

impl StructResolver for TableResolver<'_> {
    fn resolve_struct(
        &mut self,
        struct_handle: StructHandleIndex,
        _ty_args: &[SignatureToken],
    ) -> Result<InternedType> {
        // TODO: resolve cross-module struct references and intern generic
        // instantiations properly. Today the orchestrator only populates
        // entries for locally-defined, non-generic structs/enums; everything
        // else is `None`. We also return the base struct type for both
        // `SignatureToken::Struct` and `SignatureToken::StructInstantiation`
        // because instantiation interning is not yet implemented.
        //
        // Root cause: the orchestrator's struct-type table is built by the
        // layout pass, which can only handle fully-concrete, locally-defined
        // structs. Splitting *interning* from *layout computation* into two
        // phases would let us intern every type (local, cross-module, and
        // generic) up front and compute layouts lazily when the type becomes
        // fully concrete. That would also remove the `Option<InternedType>`
        // shape of `struct_types` and simplify this resolver.
        match self.struct_types.get(struct_handle.0 as usize) {
            Some(Some(ty)) => Ok(*ty),
            Some(None) => bail!(
                "unresolved struct handle {}: cross-module or generic struct references are \
                 not yet supported by the orchestrator's struct_type_table",
                struct_handle.0
            ),
            None => bail!("struct handle index {} out of bounds", struct_handle.0),
        }
    }
}
