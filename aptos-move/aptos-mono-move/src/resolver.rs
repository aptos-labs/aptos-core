// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resolves on-chain type tags into MonoMove interned types.
//!
//! A downloaded resource is keyed on-chain by a [`StructTag`]; MonoMove keys
//! global storage by an interned-type pointer. The interner deduplicates
//! structurally, and the cross-format key used by the loader for
//! `SignatureToken`s hashes identically to the one used here for `StructTag`s,
//! so the pointer this returns is the same one the interpreter produces for
//! `borrow_global<T>` during lowering. That identity is what lets a provider
//! built from these keys serve the interpreter's reads.

use anyhow::{bail, Result};
use mono_move_core::{
    types::{
        InternedType, ADDRESS_TY, BOOL_TY, I128_TY, I16_TY, I256_TY, I32_TY, I64_TY, I8_TY,
        SIGNER_TY, U128_TY, U16_TY, U256_TY, U32_TY, U64_TY, U8_TY,
    },
    Interner,
};
use mono_move_global_context::ExecutionGuard;
use move_core_types::language_storage::{StructTag, TypeTag};

/// Interns `tag` into the guard's type arena, recursing through type
/// arguments. Returns an error for type tags MonoMove cannot represent as a
/// stored value (function tags), so callers can skip such transactions.
pub fn resolve_type_tag(guard: &ExecutionGuard, tag: &TypeTag) -> Result<InternedType> {
    Ok(match tag {
        TypeTag::Bool => BOOL_TY,
        TypeTag::U8 => U8_TY,
        TypeTag::U16 => U16_TY,
        TypeTag::U32 => U32_TY,
        TypeTag::U64 => U64_TY,
        TypeTag::U128 => U128_TY,
        TypeTag::U256 => U256_TY,
        TypeTag::I8 => I8_TY,
        TypeTag::I16 => I16_TY,
        TypeTag::I32 => I32_TY,
        TypeTag::I64 => I64_TY,
        TypeTag::I128 => I128_TY,
        TypeTag::I256 => I256_TY,
        TypeTag::Address => ADDRESS_TY,
        TypeTag::Signer => SIGNER_TY,
        TypeTag::Vector(inner) => guard.vector_of(resolve_type_tag(guard, inner)?),
        TypeTag::Struct(struct_tag) => resolve_struct_tag(guard, struct_tag)?,
        TypeTag::Function(_) => bail!("function type tags are not supported"),
    })
}

/// Interns a fully-instantiated `StructTag` into the guard's type arena.
pub fn resolve_struct_tag(guard: &ExecutionGuard, tag: &StructTag) -> Result<InternedType> {
    let module_id = guard.module_id_of(&tag.address, &tag.module);
    let name = guard.identifier_of(&tag.name);
    let ty_args = tag
        .type_args
        .iter()
        .map(|t| resolve_type_tag(guard, t))
        .collect::<Result<Vec<_>>>()?;
    let ty_args = guard.type_list_of(&ty_args);
    Ok(guard.nominal_of(module_id, name, ty_args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_global_context::GlobalContext;
    use move_core_types::{
        account_address::AccountAddress, identifier::Identifier, language_storage::StructTag,
    };

    fn struct_tag(name: &str, type_args: Vec<TypeTag>) -> StructTag {
        StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("coin").unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args,
        }
    }

    /// Interning is canonical: resolving the same tag twice yields the same
    /// pointer, and a nested generic resolves to the same pointer regardless of
    /// how it is reached. (The stronger invariant — that this matches the
    /// loader's `SignatureToken`-derived pointer — is exercised end-to-end by
    /// the V2 runner.)
    #[test]
    fn resolution_is_canonical() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();

        let coin_store = struct_tag("CoinStore", vec![TypeTag::Struct(Box::new(struct_tag(
            "FakeCoin",
            vec![],
        )))]);
        let a = resolve_struct_tag(&guard, &coin_store).unwrap();
        let b = resolve_struct_tag(&guard, &coin_store).unwrap();
        // `InternedType` is a pointer; `==` is pointer identity (no `Debug`).
        assert!(
            a == b,
            "same struct tag must resolve to the same interned pointer"
        );

        // Vectors of the same element type also dedup.
        let v1 = resolve_type_tag(&guard, &TypeTag::Vector(Box::new(TypeTag::U64))).unwrap();
        let v2 = resolve_type_tag(&guard, &TypeTag::Vector(Box::new(TypeTag::U64))).unwrap();
        assert!(v1 == v2);

        // A distinct struct name resolves to a distinct pointer.
        let other = resolve_struct_tag(&guard, &struct_tag("OtherStore", vec![])).unwrap();
        assert!(a != other);
    }
}
