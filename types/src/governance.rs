// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VotingRecords {
    pub votes: AccountAddress,
}
