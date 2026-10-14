// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The Aptos framework symbols the system refers to by name.

use crate::{
    intern_struct_tag,
    interner::{InternedIdentifier, InternedModuleId},
    types::{InternedType, EMPTY_TYPE_LIST},
    Interner,
};
use aptos_types::{
    account_config::AccountResource,
    jwks::{FederatedJWKs, PatchedJWKs},
    on_chain_config::{ApprovedExecutionHashes, CurrentTimeMicroseconds, OnChainConfig},
};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::StructTag,
    move_resource::MoveStructType,
};

/// The Aptos framework symbols the VM and the transaction executor refer to,
/// interned once per global context rather than on every use.
#[derive(Debug)]
pub struct FrameworkSymbols {
    /// `0x1::block`.
    pub block: InternedModuleId,
    pub block_prologue: InternedIdentifier,
    pub block_prologue_ext: InternedIdentifier,
    pub block_prologue_ext_v2: InternedIdentifier,
    pub block_prologue_ext_v3: InternedIdentifier,
    pub block_epilogue: InternedIdentifier,

    /// `0x1::transaction_validation`.
    pub transaction_validation: InternedModuleId,
    pub versioned_prologue: InternedIdentifier,
    pub versioned_metered_prologue: InternedIdentifier,
    pub versioned_epilogue: InternedIdentifier,

    /// `0x1::object`, whose `Object<T>` arguments the VM checks against the
    /// `ObjectCore` and `T` resources under the object's address.
    pub object: InternedModuleId,
    /// `Object`.
    pub object_struct: InternedIdentifier,
    /// `0x1::object::ObjectCore`.
    pub object_core: InternedType,

    /// The on-chain configs and resources the executor reads.
    pub approved_execution_hashes: InternedType,
    /// `0x1::account::Account`, read to tell whether the sender's first
    /// transaction will create it.
    pub account_resource: InternedType,
    pub current_time_microseconds: InternedType,
    pub patched_jwks: InternedType,
    pub federated_jwks: InternedType,
}

impl FrameworkSymbols {
    /// Interns the symbols in the context behind `interner`.
    pub fn new(interner: &impl Interner) -> Self {
        let module = |name| interner.module_id_of(&AccountAddress::ONE, name);
        let identifier = |name| interner.identifier_of(name);
        let object = module(ident_str!("object"));
        Self {
            block: module(ident_str!("block")),
            block_prologue: identifier(ident_str!("block_prologue")),
            block_prologue_ext: identifier(ident_str!("block_prologue_ext")),
            block_prologue_ext_v2: identifier(ident_str!("block_prologue_ext_v2")),
            block_prologue_ext_v3: identifier(ident_str!("block_prologue_ext_v3")),
            block_epilogue: identifier(ident_str!("block_epilogue")),

            transaction_validation: module(ident_str!("transaction_validation")),
            versioned_prologue: identifier(ident_str!("versioned_prologue")),
            versioned_metered_prologue: identifier(ident_str!("versioned_metered_prologue")),
            versioned_epilogue: identifier(ident_str!("versioned_epilogue")),

            object,
            object_struct: identifier(ident_str!("Object")),
            object_core: interner.nominal_of(
                object,
                identifier(ident_str!("ObjectCore")),
                EMPTY_TYPE_LIST,
            ),

            approved_execution_hashes: config_type::<ApprovedExecutionHashes>(interner),
            account_resource: resource_type::<AccountResource>(interner),
            current_time_microseconds: config_type::<CurrentTimeMicroseconds>(interner),
            patched_jwks: config_type::<PatchedJWKs>(interner),
            federated_jwks: resource_type::<FederatedJWKs>(interner),
        }
    }
}

/// Interns the type of the on-chain config `T`.
fn config_type<T: OnChainConfig>(interner: &impl Interner) -> InternedType {
    framework_type(&T::struct_tag(), interner)
}

/// Interns the type of the framework resource `T`.
fn resource_type<T: MoveStructType>(interner: &impl Interner) -> InternedType {
    framework_type(&T::struct_tag(), interner)
}

fn framework_type(tag: &StructTag, interner: &impl Interner) -> InternedType {
    intern_struct_tag(tag, interner).expect("a framework struct tag is a valid, non-generic type")
}
