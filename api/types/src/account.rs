// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{HexEncodedBytes, U64};
use aptos_types::account_config::AccountResource;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Account data
///
/// A simplified version of the onchain Account resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: HexEncodedBytes,
    // Stateless accounts donot have `0x1::Account` resource.
    // If the `0x1::Account` resource doesn't exist, the above sequence_number is set to 0,
    // and the below state_exists is set to false. 
    pub state_exists: bool,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        let authentication_key: HexEncodedBytes = ar.authentication_key().to_vec().into();
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key,
            state_exists: true,
        }
    }
}
