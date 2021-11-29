// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{HexEncodedBytes, U64};

use diem_types::account_config::AccountResource;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountData {
    pub sequence_number: U64,
    pub authentication_key: HexEncodedBytes,
}

impl From<AccountResource> for AccountData {
    fn from(ar: AccountResource) -> Self {
        Self {
            sequence_number: ar.sequence_number().into(),
            authentication_key: ar.authentication_key().to_vec().into(),
        }
    }
}
