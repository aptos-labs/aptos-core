// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub trait CoinType {
    fn type_tag() -> TypeTag;

    fn coin_info_address() -> AccountAddress;
}

static APTOS_COIN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("aptos_coin").to_owned(),
        name: ident_str!("AptosCoin").to_owned(),
        type_args: vec![],
    }))
});

#[derive(Debug, Serialize, Deserialize)]
pub struct AptosCoinType;

impl CoinType for AptosCoinType {
    fn type_tag() -> TypeTag {
        APTOS_COIN_TYPE.clone()
    }

    fn coin_info_address() -> AccountAddress {
        AccountAddress::ONE
    }
}

impl MoveStructType for AptosCoinType {
    const MODULE_NAME: &'static IdentStr = ident_str!("aptos_coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("AptosCoin");
}

pub static DUMMY_COIN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("dummy_coin").to_owned(),
        name: ident_str!("DummyCoin").to_owned(),
        type_args: vec![],
    }))
});

pub struct DummyCoinType;
impl CoinType for DummyCoinType {
    fn type_tag() -> TypeTag {
        DUMMY_COIN_TYPE.clone()
    }

    fn coin_info_address() -> AccountAddress {
        AccountAddress::ONE
    }
}
