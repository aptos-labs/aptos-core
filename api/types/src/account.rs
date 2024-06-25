// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{HexEncodedBytes, U64};
use aptos_types::{account_config::{AccountResource, lite_account}};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Account data
///
/// A simplified version of the onchain Account resource
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: Option<HexEncodedBytes>,
    pub authentication_function_info: Option<String>,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        let authentication_key: Option<HexEncodedBytes> = Some(ar.authentication_key().to_vec().into());
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key,
            authentication_function_info: None,
        }
    }
}

impl From<lite_account::LiteAccountGroup> for AccountData {
    fn from(lag: lite_account::LiteAccountGroup) -> Self {
        let authentication_key: Option<HexEncodedBytes> = lag.authentication_key().map(|key| key.to_vec().into());
        Self {
            sequence_number: lag.sequence_number().into(),
            authentication_key,
            authentication_function_info: lag.dispatchable_authenticator.map(|r| r.auth_function.to_string()),
        }
    }
}
