// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Types and identifiers for parsing Move structs and types

use aptos_sdk::move_types::{ident_str, identifier::Identifier};

// Module identifiers
pub fn aptos_coin_module_identifier() -> Identifier {
    ident_str!("aptos_coin").into()
}

pub fn account_module_identifier() -> Identifier {
    ident_str!("account").into()
}

pub fn coin_module_identifier() -> Identifier {
    ident_str!("coin").into()
}

pub fn stake_module_identifier() -> Identifier {
    ident_str!("stake").into()
}

// Resource Identifiers
pub fn account_resource_identifier() -> Identifier {
    ident_str!("Account").into()
}

pub fn coin_info_resource_identifier() -> Identifier {
    ident_str!("CoinInfo").into()
}

pub fn coin_store_resource_identifier() -> Identifier {
    ident_str!("CoinStore").into()
}

pub fn aptos_coin_resource_identifier() -> Identifier {
    ident_str!("AptosCoin").into()
}

pub fn stake_pool_resource_identifier() -> Identifier {
    ident_str!("StakePool").into()
}

// Function identifiers
// Function identifiers
pub fn create_account_function_identifier() -> Identifier {
    ident_str!("create_account").into()
}

pub fn transfer_function_identifier() -> Identifier {
    ident_str!("transfer").into()
}

pub fn set_operator_function_identifier() -> Identifier {
    ident_str!("set_operator").into()
}

// Field identifiers
pub fn decimals_field_identifier() -> Identifier {
    ident_str!("decimals").into()
}

pub fn deposit_events_field_identifier() -> Identifier {
    ident_str!("deposit_events").into()
}

pub fn withdraw_events_field_identifier() -> Identifier {
    ident_str!("withdraw_events").into()
}

pub fn set_operator_events_field_identifier() -> Identifier {
    ident_str!("set_operator_events").into()
}

pub fn sequence_number_field_identifier() -> Identifier {
    ident_str!("sequence_number").into()
}

pub fn symbol_field_identifier() -> Identifier {
    ident_str!("symbol").into()
}
