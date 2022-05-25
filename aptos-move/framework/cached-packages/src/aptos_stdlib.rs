// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]
include!(concat!(env!("OUT_DIR"), "/transaction_script_builder.rs"));

use aptos_types::utility_coin::TEST_COIN_TYPE;
use move_deps::move_core_types::language_storage::{StructTag, TypeTag};

pub fn encode_test_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(AccountAddress::ONE, ident_str!("Coin").to_owned()),
        ident_str!("transfer").to_owned(),
        vec![TEST_COIN_TYPE.clone()],
        vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
    ))
}
