// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ExtendRef {
    #[serde(rename = "self")]
    pub address: AccountAddress,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct PublisherRef {
    pub extend_ref: ExtendRef,
}

impl PublisherRef {
    pub fn new(address: AccountAddress) -> Self {
        PublisherRef {
            extend_ref: ExtendRef { address },
        }
    }
}
