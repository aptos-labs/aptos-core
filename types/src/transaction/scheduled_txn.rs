// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::as_move_value::AsMoveValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, CORE_CODE_ADDRESS},
    value::{MoveStruct, MoveValue},
};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduleMapKey {
    pub time: u64,
    pub gas_priority: u64,
    /// SHA3-256
    pub txn_id: Vec<u8>,
}

impl AsMoveValue for ScheduleMapKey {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.time.as_move_value(),
            self.gas_priority.as_move_value(),
            self.txn_id.as_move_value(),
        ]))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ScheduledTransactionInfoWithKey {
    pub sender_handle: AccountAddress,
    /// Maximum gas to spend for this transaction
    pub max_gas_amount: u64,
    /// Charged @ lesser of {max_gas_unit_price, max_gas_unit_price other than this in the block executed}
    pub max_gas_unit_price: u64,
    /// To be determined during execution
    pub gas_unit_price_charged: u64,
    /// Key to the scheduled txn in the Schedule queue
    pub key: ScheduleMapKey,
}

pub static SCHEDULED_TRANSACTIONS_MODULE_INFO: Lazy<ScheduledTxnsModuleInfo> =
    Lazy::new(|| ScheduledTxnsModuleInfo {
        module_addr: CORE_CODE_ADDRESS,
        module_name: Identifier::new("scheduled_txns").unwrap(),
        get_ready_transactions_name: Identifier::new("get_ready_transactions").unwrap(),
        execute_user_function_wrapper_name: Identifier::new("execute_user_function_wrapper")
            .unwrap(),
    });

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
