// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RoyaltyMutate {
    creator: AccountAddress,
    collection: String,
    token: String,
    old_royalty_numerator: u64,
    old_royalty_denominator: u64,
    old_royalty_payee_addr: AccountAddress,
    new_royalty_numerator: u64,
    new_royalty_denominator: u64,
    new_royalty_payee_addr: AccountAddress,
}

impl RoyaltyMutate {
    pub fn new(
        creator: AccountAddress,
        collection: String,
        token: String,
        old_royalty_numerator: u64,
        old_royalty_denominator: u64,
        old_royalty_payee_addr: AccountAddress,
        new_royalty_numerator: u64,
        new_royalty_denominator: u64,
        new_royalty_payee_addr: AccountAddress,
    ) -> Self {
        Self {
            creator,
            collection,
            token,
            old_royalty_numerator,
            old_royalty_denominator,
            old_royalty_payee_addr,
            new_royalty_numerator,
            new_royalty_denominator,
            new_royalty_payee_addr,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn creator(&self) -> &AccountAddress {
        &self.creator
    }

    pub fn collection(&self) -> &String {
        &self.collection
    }

    pub fn token(&self) -> &String {
        &self.token
    }

    pub fn old_royalty_numerator(&self) -> &u64 {
        &self.old_royalty_numerator
    }

    pub fn old_royalty_denominator(&self) -> &u64 {
        &self.old_royalty_denominator
    }

    pub fn old_royalty_payee_addr(&self) -> &AccountAddress {
        &self.old_royalty_payee_addr
    }

    pub fn new_royalty_numerator(&self) -> &u64 {
        &self.new_royalty_numerator
    }

    pub fn new_royalty_denominator(&self) -> &u64 {
        &self.new_royalty_denominator
    }

    pub fn new_royalty_payee_addr(&self) -> &AccountAddress {
        &self.new_royalty_payee_addr
    }
}

impl MoveStructType for RoyaltyMutate {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("RoyaltyMutate");
}

impl MoveEventV2Type for RoyaltyMutate {}

pub static ROYALTY_MUTATE_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("RoyaltyMutate").to_owned(),
        type_args: vec![],
    }))
});
