// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use anyhow::Result;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A Rust representation of an Account resource.
/// This is not how the Account is represented in the VM but it's a convenient representation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct LiteAccountGroup {
    pub account: AccountResource,
    pub authenticator: Authenticator,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountResource {
    pub sequence_number: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct NativeAuthenticatorResource {
    authentication_key: Vec<u8>,
}

impl From<Vec<u8>> for NativeAuthenticatorResource {
    fn from(authentication_key: Vec<u8>) -> Self {
        Self { authentication_key }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct CustomizedAuthenticatorResource {
    account_address: AccountAddress,
    module_name: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum Authenticator {
    Native(NativeAuthenticatorResource),
    Customized(CustomizedAuthenticatorResource),
}

impl LiteAccountGroup {
    /// Constructs an Account resource.
    pub fn new(sequence_number: u64, authenticator: Authenticator) -> Self {
        LiteAccountGroup {
            account: AccountResource { sequence_number },
            authenticator,
        }
    }

    /// Return the sequence_number field for the given Account
    pub fn sequence_number(&self) -> u64 {
        self.account.sequence_number
    }

    /// Return the authentication_key field for the given Account
    pub fn authentication_key(&self) -> &[u8] {
        match &self.authenticator {
            Authenticator::Native(native_authenticator) => {
                native_authenticator.authentication_key.as_slice()
            },
            _ => unreachable!(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut group = BTreeMap::new();
        group.insert(
            AccountResource::struct_tag(),
            bcs::to_bytes(&self.account).unwrap(),
        );
        match &self.authenticator {
            Authenticator::Native(native) => group.insert(
                NativeAuthenticatorResource::struct_tag(),
                bcs::to_bytes(native)?,
            ),
            Authenticator::Customized(customized) => group.insert(
                CustomizedAuthenticatorResource::struct_tag(),
                bcs::to_bytes(customized)?,
            ),
        };
        Ok(bcs::to_bytes(&group)?)
    }
}

impl MoveStructType for LiteAccountGroup {
    const MODULE_NAME: &'static IdentStr = ident_str!("lite_account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("LiteAccountGroup");
}

impl MoveResource for LiteAccountGroup {}

impl MoveStructType for AccountResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("lite_account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Account");
}

impl MoveResource for AccountResource {}

impl MoveStructType for NativeAuthenticatorResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("lite_account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("NativeAuthenticator");
}

impl MoveResource for NativeAuthenticatorResource {}

impl MoveStructType for CustomizedAuthenticatorResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("lite_account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CustomizedAuthenticator");
}

impl MoveResource for CustomizedAuthenticatorResource {}
