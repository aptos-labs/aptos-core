// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction::ChangeSet;
use aptos_vm_genesis::generate_genesis_change_set_for_testing;
use once_cell::sync::Lazy;

pub static GENESIS_CHANGE_SET_HEAD: Lazy<ChangeSet> =
    Lazy::new(generate_genesis_change_set_for_testing);
