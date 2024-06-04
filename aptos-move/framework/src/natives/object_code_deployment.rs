// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::event::EventHandle;
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ExtendRef {
    #[serde(rename = "self")]
    pub address: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ManagingRefs {
    pub extend_ref: ExtendRef,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ObjectCore {
    pub guid_creation_num: u64,
    pub owner: AccountAddress,
    pub allow_ungated_transfer: bool,
    pub transfer_events: EventHandle,
}

impl ManagingRefs {
    pub fn new(address: AccountAddress) -> Self {
        ManagingRefs {
            extend_ref: ExtendRef { address },
        }
    }
}
