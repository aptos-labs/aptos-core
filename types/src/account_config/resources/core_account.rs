// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::account_config::constants::CORE_ACCOUNT_MODULE_IDENTIFIER;
use move_core_types::{
    account_address::AccountAddress,
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
}

impl AccountResource {
    /// Constructs an Account resource.
    pub fn new(
        sequence_number: u64,
        authentication_key: Vec<u8>,
        self_address: AccountAddress,
    ) -> Self {
        AccountResource {
            authentication_key,
            sequence_number,
            self_address,
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
}

impl MoveStructType for AccountResource {
    const MODULE_NAME: &'static IdentStr = CORE_ACCOUNT_MODULE_IDENTIFIER;
    const STRUCT_NAME: &'static IdentStr = CORE_ACCOUNT_MODULE_IDENTIFIER;
}

impl MoveResource for AccountResource {}
