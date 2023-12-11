// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::intrinsics::abort;
use crate::{account_address::AccountAddress, event::EventHandle};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// A Rust representation of an Account resource.
/// This is not how the Account is represented in the VM but it's a convenient representation.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct LiteAccount {
    pub account_resource: AccountResource,
    pub authenticator: Authenticator,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountResource {
    sequence_number: u64,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct NativeAuthenticatorResource {
    authentication_key: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
struct CustomizedAuthenticatorResource {
    account_address: AccountAddress,
    module_name: Vec<u8>,
}

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum Authenticator {
    Native(NativeAuthenticatorResource),
    Customized(CustomizedAuthenticatorResource),
}

impl LiteAccount {
    /// Constructs an Account resource.
    pub fn new(
        sequence_number: u64,
        authentication_key: Vec<u8>,
    ) -> Self {
        LiteAccount {
            account_resource: AccountResource {
                sequence_number,
            },
            authenticator: Authenticator::Native(NativeAuthenticatorResource {
                authentication_key
            }),
        }
    }

    /// Return the sequence_number field for the given AccountResource
    pub fn sequence_number(&self) -> u64 {
        self.account_resource.sequence_number
    }

    /// Return the authentication_key field for the given AccountResource
    pub fn authentication_key(&self) -> &[u8] {
        match &self.authenticator {
            Authenticator::Native(native_authenticator_resource) => native_authenticator_resource.authentication_key.as_slice(),
            _ => abort(),
        }
    }
}

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
