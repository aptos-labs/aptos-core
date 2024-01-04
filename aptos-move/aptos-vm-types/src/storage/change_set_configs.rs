// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::VMChangeSet, check_change_set::CheckChangeSet};
use aptos_gas_schedule::AptosGasParameters;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;

#[derive(Clone, Debug)]
pub struct ChangeSetConfigs {
    gas_feature_version: u64,
    max_bytes_per_write_op: u64,
    max_bytes_all_write_ops_per_transaction: u64,
    max_bytes_per_event: u64,
    max_bytes_all_events_per_transaction: u64,
    max_write_ops_per_transaction: u64,
}

impl ChangeSetConfigs {
    pub fn unlimited_at_gas_feature_version(gas_feature_version: u64) -> Self {
        Self::new_impl(
            gas_feature_version,
            u64::MAX,
            u64::MAX,
            u64::MAX,
            u64::MAX,
            u64::MAX,
        )
    }

    pub fn new(feature_version: u64, gas_params: &AptosGasParameters) -> Self {
        if feature_version >= 5 {
            Self::from_gas_params(feature_version, gas_params)
        } else if feature_version >= 3 {
            Self::for_feature_version_3()
        } else {
            Self::unlimited_at_gas_feature_version(feature_version)
        }
    }

    fn new_impl(
        gas_feature_version: u64,
        max_bytes_per_write_op: u64,
        max_bytes_all_write_ops_per_transaction: u64,
        max_bytes_per_event: u64,
        max_bytes_all_events_per_transaction: u64,
        max_write_ops_per_transaction: u64,
    ) -> Self {
        Self {
            gas_feature_version,
            max_bytes_per_write_op,
            max_bytes_all_write_ops_per_transaction,
            max_bytes_per_event,
            max_bytes_all_events_per_transaction,
            max_write_ops_per_transaction,
        }
    }

    pub fn legacy_resource_creation_as_modification(&self) -> bool {
        // Bug fixed at gas_feature_version 3 where (non-group) resource creation was converted to
        // modification.
        // Modules and table items were not affected (https://github.com/aptos-labs/aptos-core/pull/4722/commits/7c5e52297e8d1a6eac67a68a804ab1ca2a0b0f37).
        // Resource groups and state values with metadata were not affected because they were
        // introduced later than feature_version 3 on all networks.
        self.gas_feature_version < 3
    }

    fn for_feature_version_3() -> Self {
        const MB: u64 = 1 << 20;

        Self::new_impl(3, MB, u64::MAX, MB, 10 * MB, u64::MAX)
    }

    fn from_gas_params(gas_feature_version: u64, gas_params: &AptosGasParameters) -> Self {
        let params = &gas_params.vm.txn;
        Self::new_impl(
            gas_feature_version,
            params.max_bytes_per_write_op.into(),
            params.max_bytes_all_write_ops_per_transaction.into(),
            params.max_bytes_per_event.into(),
            params.max_bytes_all_events_per_transaction.into(),
            params.max_write_ops_per_transaction.into(),
        )
    }
}

impl CheckChangeSet for ChangeSetConfigs {
    fn check_change_set(&self, change_set: &VMChangeSet) -> PartialVMResult<()> {
        if self.max_write_ops_per_transaction != 0
            && change_set.num_write_ops() as u64 > self.max_write_ops_per_transaction
        {
            return Err(PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED)
                .with_message("Too many write ops.".to_string()));
        }

        let mut write_set_size = 0;
        for (key, op_size) in change_set.write_set_size_iter() {
            if let Some(len) = op_size.write_len() {
                let write_op_size = len + (key.size() as u64);
                if write_op_size > self.max_bytes_per_write_op {
                    return Err(PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
                }
                write_set_size += write_op_size;
            }
            if write_set_size > self.max_bytes_all_write_ops_per_transaction {
                return Err(PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
            }
        }

        let mut total_event_size = 0;
        for (event, _) in change_set.events() {
            let size = event.event_data().len() as u64;
            if size > self.max_bytes_per_event {
                return Err(PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
            }
            total_event_size += size;
            if total_event_size > self.max_bytes_all_events_per_transaction {
                return Err(PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED));
            }
        }

        Ok(())
    }
}
