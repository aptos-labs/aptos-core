// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// A enum that captures the collection of role-specific resources stored under each account type
#[derive(Debug, Serialize, Deserialize)]
pub enum AccountRole {
    TreasuryCompliance,
    Unknown,
}
