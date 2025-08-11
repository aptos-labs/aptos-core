// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    event::{EventHandle, EventKey},
};
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
pub struct AccountResource {
    authentication_key: Vec<u8>,
    pub sequence_number: u64,
    guid_creation_num: u64,
    coin_register_events: EventHandle,
    key_rotation_events: EventHandle,
    rotation_capability_offer: Option<AccountAddress>,
    signer_capability_offer: Option<AccountAddress>,
}

impl AccountResource {
    /// Constructs an Account resource.
    pub fn new(
        sequence_number: u64,
        authentication_key: Vec<u8>,
        coin_register_events: EventHandle,
        key_rotation_events: EventHandle,
    ) -> Self {
        AccountResource {
            authentication_key,
            sequence_number,
            guid_creation_num: 0,
            coin_register_events,
            key_rotation_events,
            rotation_capability_offer: None,
            signer_capability_offer: None,
        }
    }

    pub fn new_stateless(address: AccountAddress) -> Self {
        AccountResource {
            authentication_key: bcs::to_bytes(&address).unwrap(),
            sequence_number: 0,
            guid_creation_num: 2,
            coin_register_events: EventHandle::new(EventKey::new(0, address), 0),
            key_rotation_events: EventHandle::new(EventKey::new(1, address), 0),
            rotation_capability_offer: None,
            signer_capability_offer: None,
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

    pub fn coin_register_events(&self) -> &EventHandle {
        &self.coin_register_events
    }

    pub fn key_rotation_events(&self) -> &EventHandle {
        &self.key_rotation_events
    }

    pub fn guid_creation_num(&self) -> u64 {
        self.guid_creation_num
    }

    pub fn rotation_capability_offer(&self) -> Option<AccountAddress> {
        self.rotation_capability_offer
    }

    pub fn signer_capability_offer(&self) -> Option<AccountAddress> {
        self.signer_capability_offer
    }
}

impl MoveStructType for AccountResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Account");
}

impl MoveResource for AccountResource {}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_account_resource_has_special_address() {
        // Note: module loading gas charging logic depends on this assumption. This should never
        // change, but a test should catch if address changes at any point.
        assert!(AccountResource::struct_tag().address.is_special());
    }
}
