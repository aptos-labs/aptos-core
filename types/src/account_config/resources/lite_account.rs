// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::function_info::FunctionInfo;
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::StructTag,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use crate::account_config::ObjectGroupResource;

/// A Rust representation of an Account resource.
/// This is not how the Account is represented in the VM but it's a convenient representation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct LiteAccountGroup {
    addr: AccountAddress,
    pub account: Option<AccountResource>,
    pub native_authenticator: Option<NativeAuthenticatorResource>,
    pub dispatchable_authenticator: Option<DispatchableAuthenticatorResource>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct AccountResource {
    pub sequence_number: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct NativeAuthenticatorResource {
    authentication_key: Option<Vec<u8>>,
}

impl From<Option<Vec<u8>>> for NativeAuthenticatorResource {
    fn from(authentication_key: Option<Vec<u8>>) -> Self {
        Self { authentication_key }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct DispatchableAuthenticatorResource {
    pub auth_function: FunctionInfo,
}

impl LiteAccountGroup {
    /// Constructs an Account resource.
    pub fn new(
        addr: AccountAddress,
        sequence_number: Option<u64>,
        auth_key: Option<Option<Vec<u8>>>,
        dispatchable_authenticator: Option<DispatchableAuthenticatorResource>,
    ) -> Self {
        LiteAccountGroup {
            addr,
            account: sequence_number.map(|s| AccountResource { sequence_number: s }),
            native_authenticator: auth_key.map(|k| NativeAuthenticatorResource {
                authentication_key: k,
            }),
            dispatchable_authenticator,
        }
    }

    /// Return the sequence_number field for the given Account
    pub fn sequence_number(&self) -> u64 {
        if let Some(ar) = &self.account {
            ar.sequence_number
        } else {
            0
        }
    }

    /// Return the authentication_key field for the given Account
    pub fn authentication_key(&self) -> Option<&[u8]> {
        if let Some(na) = &self.native_authenticator {
            na.authentication_key.as_deref()
        } else {
            Some(self.addr.as_ref())
        }
    }

    pub fn add_to_object_group(&self, group: &mut ObjectGroupResource) {
        if let Some(ar) = &self.account {
            group.insert(AccountResource::struct_tag(), bcs::to_bytes(ar).unwrap());
        }
        if let Some(na) = &self.native_authenticator {
            group.insert(
                NativeAuthenticatorResource::struct_tag(),
                bcs::to_bytes(na).unwrap(),
            );
        }
        if let Some(da) = &self.dispatchable_authenticator {
            group.insert(
                DispatchableAuthenticatorResource::struct_tag(),
                bcs::to_bytes(da).unwrap(),
            );
        }
    }

    pub fn from_bytes(addr: &AccountAddress, value: Option<&[u8]>) -> Result<Self> {
        if let Some(value) = value {
            let group: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(value)?;
            let account = group
                .get(&AccountResource::struct_tag())
                .map(|bytes| bcs::from_bytes::<AccountResource>(bytes.as_slice()))
                .transpose()?;
            let native_authenticator = group
                .get(&NativeAuthenticatorResource::struct_tag())
                .map(|bytes| bcs::from_bytes::<NativeAuthenticatorResource>(bytes.as_slice()))
                .transpose()?;
            let dispatchable_authenticator = group
                .get(&DispatchableAuthenticatorResource::struct_tag())
                .map(|bytes| bcs::from_bytes::<DispatchableAuthenticatorResource>(bytes.as_slice()))
                .transpose()?;
            Ok(Self {
                addr: *addr,
                account,
                native_authenticator,
                dispatchable_authenticator,
            })
        } else {
            Ok(Self::new(*addr, None, None, None))
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

impl MoveStructType for DispatchableAuthenticatorResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("lite_account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DispatchableAuthenticator");
}

impl MoveResource for DispatchableAuthenticatorResource {}
