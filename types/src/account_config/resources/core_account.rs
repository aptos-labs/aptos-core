// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventHandle;
use move_deps::move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// A Rust representation of an Account resource.
/// This is not how the Account is represented in the VM but it's a convenient representation.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountResource {
    authentication_key: Vec<u8>,
    sequence_number: u64,
    self_address: AccountAddress,
    coin_register_events: EventHandle,
}

impl AccountResource {
    /// Constructs an Account resource.
    pub fn new(
        sequence_number: u64,
        authentication_key: Vec<u8>,
        self_address: AccountAddress,
        coin_register_events: EventHandle,
    ) -> Self {
        AccountResource {
            authentication_key,
            sequence_number,
            self_address,
            coin_register_events,
        }
    }

    /// Return the sequence_number field for the given AccountResource
    pub fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    /// Return the authentication_key field for the given AccountResource
    pub fn authentication_key(&self) -> &[u8] {
        &self.authentication_key
    }

    pub fn address(&self) -> AccountAddress {
        self.self_address
    }

    pub fn coin_register_events(&self) -> &EventHandle {
        &self.coin_register_events
    }
}

impl MoveStructType for AccountResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Account");
}

impl MoveResource for AccountResource {}
