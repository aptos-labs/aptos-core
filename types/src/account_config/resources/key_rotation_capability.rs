// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress, account_config::constants::APTOS_ACCOUNT_MODULE_IDENTIFIER,
};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct KeyRotationCapabilityResource {
    account_address: AccountAddress,
}

impl MoveStructType for KeyRotationCapabilityResource {
    const MODULE_NAME: &'static IdentStr = APTOS_ACCOUNT_MODULE_IDENTIFIER;
    const STRUCT_NAME: &'static IdentStr = ident_str!("KeyRotationCapability");
}

impl MoveResource for KeyRotationCapabilityResource {}
