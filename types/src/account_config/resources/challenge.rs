// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

// This struct includes TypeInfo (account_address, module_name, and struct_name)
// and RotationProofChallenge-specific information (sequence_number, originator, current_auth_key, and new_public_key)
// Since the struct RotationProofChallenge is defined in "0x1::account::RotationProofChallenge",
// we will be passing in "0x1" to `account_address`, "account" to `module_name`, and "RotationProofChallenge" to `struct_name`
// Originator refers to the user's address
#[derive(Serialize, Deserialize)]
pub struct RotationProofChallenge {
    // Should be `CORE_CODE_ADDRESS`
    pub account_address: AccountAddress,
    // Should be `account`
    pub module_name: String,
    // Should be `RotationProofChallenge`
    pub struct_name: String,
    pub sequence_number: u64,
    pub originator: AccountAddress,
    pub current_auth_key: AccountAddress,
    pub new_public_key: Vec<u8>,
}
