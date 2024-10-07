// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
};
use once_cell::sync::Lazy;

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

pub struct AptosCoinType;

impl CoinType for AptosCoinType {
    fn type_tag() -> TypeTag {
        APTOS_COIN_TYPE.clone()
    }

    fn coin_info_address() -> AccountAddress {
        AccountAddress::ONE
    }
}
