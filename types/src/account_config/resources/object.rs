// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::event::EventHandle;
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

/// A Rust representation of ObjectGroup.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ObjectGroupResource {
    pub group: BTreeMap<StructTag, Vec<u8>>,
}

impl ObjectGroupResource {
    pub fn insert(&mut self, key: StructTag, value: Vec<u8>) -> Option<Vec<u8>> {
        self.group.insert(key, value)
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self.group)?)
    }
}

impl MoveStructType for ObjectGroupResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("object");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ObjectGroup");
}

impl MoveResource for ObjectGroupResource {}

/// A Rust representation of ObjectCoreResource.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ObjectCoreResource {
    pub guid_creation_num: u64,
    pub owner: AccountAddress,
    pub allow_ungated_transfer: bool,
    pub transfer_events: EventHandle,
}

impl ObjectCoreResource {
    pub fn new(
        owner: AccountAddress,
        allow_ungated_transfer: bool,
        transfer_events: EventHandle,
    ) -> Self {
        Self {
            guid_creation_num: 0,
            owner,
            allow_ungated_transfer,
            transfer_events,
        }
    }

    pub fn transfer_events(&self) -> &EventHandle {
        &self.transfer_events
    }
}

impl MoveStructType for ObjectCoreResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("object");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ObjectCore");
}

impl MoveResource for ObjectCoreResource {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Object {
    inner: AccountAddress,
}

impl Object {
    pub fn new(inner: AccountAddress) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &AccountAddress {
        &self.inner
    }
}
