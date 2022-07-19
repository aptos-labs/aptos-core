// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

include!(concat!(
    concat!(env!("OUT_DIR"), "/framework"),
    "/transaction_script_builder.rs"
));

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

pub fn encode_create_resource_account(
    seed: &str,
    authentication_key: Option<AccountAddress>,
) -> TransactionPayload {
    let seed: Vec<u8> = bcs::to_bytes(seed).unwrap();
    let authentication_key: Vec<u8> = if let Some(key) = authentication_key {
        bcs::to_bytes(&key).unwrap()
    } else {
        vec![]
    };
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::ONE,
            ident_str!("ResourceAccount").to_owned(),
        ),
        ident_str!("create_resource_account").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&seed).unwrap(),
            bcs::to_bytes(&authentication_key).unwrap(),
        ],
    ))
}
