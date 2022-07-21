// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use aptos_types::{
    account_address::AccountAddress,
    transaction::{Script, ScriptFunction, TransactionArgument, TransactionPayload, VecBytes},
    utility_coin::APTOS_COIN_TYPE,
};
use move_deps::move_core_types::language_storage::{StructTag, TypeTag};
use move_deps::move_core_types::{ident_str, language_storage::ModuleId};

pub use crate::generated_aptos_txn_builder::*;

/* Currently the generated builders are checked in as source.

include!(concat!(
    concat!(env!("OUT_DIR"), "/framework"),
    "/transaction_script_builder.rs"
));
 */

pub fn encode_aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned()),
        ident_str!("transfer").to_owned(),
        vec![APTOS_COIN_TYPE.clone()],
        vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
    ))
}
