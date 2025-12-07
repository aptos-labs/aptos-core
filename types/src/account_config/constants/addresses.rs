// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::account_address::AccountAddress;
pub use move_core_types::language_storage::{CORE_CODE_ADDRESS, EXPERIMENTAL_CODE_ADDRESS};

pub fn aptos_test_root_address() -> AccountAddress {
    AccountAddress::from_hex_literal("0xA550C18")
        .expect("Parsing valid hex literal should always succeed")
}

pub fn reserved_vm_address() -> AccountAddress {
    AccountAddress::new([0u8; AccountAddress::LENGTH])
}
