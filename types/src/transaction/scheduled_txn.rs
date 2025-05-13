// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use move_core_types::identifier::Identifier;
use move_core_types::language_storage::{CORE_CODE_ADDRESS, ModuleId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduledTransaction {
    pub sender_handle: AccountAddress,
    /// 100ms granularity
    pub scheduled_time: u64,
    /// Maximum gas to spend for this transaction
    pub max_gas_amount: u64,
    /// Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
    pub max_gas_unit_price: u64,
    /// Option to pass a signer when f is called
    pub pass_signer: bool,
    /// BCS serialized function, we cannot deserialize move closure in rust
    pub f: Vec<u8>
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

pub static SCHEDULED_TRANSACTIONS_MODULE_INFO: Lazy<ScheduledTxnsModuleInfo> =
    Lazy::new(||
        ScheduledTxnsModuleInfo {
            module_addr: CORE_CODE_ADDRESS,
            module_name: Identifier::new("scheduled_txns").unwrap(),
            get_ready_transactions_name: Identifier::new("get_ready_transactions").unwrap(),
            execute_user_function_wrapper_name: Identifier::new("execute_user_function_wrapper").unwrap(),
        }
    );

pub struct ScheduledTxnsModuleInfo {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub get_ready_transactions_name: Identifier,
    pub execute_user_function_wrapper_name: Identifier,
}

impl ScheduledTxnsModuleInfo {
    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.module_addr, self.module_name.clone())
    }
}
