// Copyright (c) Aptos
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
    /// Next sequence number of the account
    ///
    /// This will be the sequence number of the next transaction committed on this account
    pub sequence_number: U64,
    /// Authentication key
    ///
    /// A SHA-256 of public keys and authentication scheme of the account
    pub authentication_key: HexEncodedBytes,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        let authentication_key: HexEncodedBytes = ar.authentication_key().to_vec().into();
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key,
        }
    }
}
