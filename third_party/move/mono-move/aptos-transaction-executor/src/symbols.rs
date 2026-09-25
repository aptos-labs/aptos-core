// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    Interner,
};
use move_core_types::{account_address::AccountAddress, ident_str};

/// The framework symbols the executor refers to, interned once per global
/// context.
pub(crate) struct FrameworkSymbols {
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
    pub versioned_epilogue: InternedIdentifier,
}

impl FrameworkSymbols {
    /// Interns the symbols in the context behind `interner`.
    pub(crate) fn new(interner: &impl Interner) -> Self {
        let module = |name| interner.module_id_of(&AccountAddress::ONE, name);
        let function = |name| interner.identifier_of(name);
        Self {
            block: module(ident_str!("block")),
            block_prologue: function(ident_str!("block_prologue")),
            block_prologue_ext: function(ident_str!("block_prologue_ext")),
            block_prologue_ext_v2: function(ident_str!("block_prologue_ext_v2")),
            block_prologue_ext_v3: function(ident_str!("block_prologue_ext_v3")),
            block_epilogue: function(ident_str!("block_epilogue")),

            transaction_validation: module(ident_str!("transaction_validation")),
            versioned_prologue: function(ident_str!("versioned_prologue")),
            versioned_epilogue: function(ident_str!("versioned_epilogue")),
        }
    }
}
