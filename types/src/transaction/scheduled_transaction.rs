// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::EntryFunction;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduledTransaction {
    pub scheduled_time: u64,
    pub max_gas_unit: u64,
    pub sender: AccountAddress,
    pub payload: EntryFunction,
}
