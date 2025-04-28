// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduledTransaction {
    sender_handle: AccountAddress,
    /// 100ms granularity
    scheduled_time: u64,
    /// Maximum gas to spend for this transaction
    max_gas_amount: u64,
    /// Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
    max_gas_unit_price: u64,
    /// Option to pass a signer when f is called
    pass_signer: bool,
    /// BCS serialized function, we cannot deserialize move closure in rust
    f: Vec<u8>
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduleMapKey {
    time: u64,
    gas_priority: u64,
    /// SHA3-256
    txn_id: Vec<u8>
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduledTransactionWithKey {
    pub txn: ScheduledTransaction,
    pub key: ScheduleMapKey
}
