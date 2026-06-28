// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test helpers shared by the runtime integration tests.

/// Builds an interned module id for hand-built test functions.
#[macro_export]
macro_rules! program_module_id {
    ($name:literal) => {{
        static MODULE_ID: ::mono_move_core::interner::ModuleId =
            ::mono_move_core::interner::ModuleId::new(
                ::move_core_types::account_address::AccountAddress::ONE,
                ::mono_move_alloc::GlobalArenaPtr::from_static($name),
            );
        ::mono_move_alloc::GlobalArenaPtr::from_static(&MODULE_ID)
    }};
}
