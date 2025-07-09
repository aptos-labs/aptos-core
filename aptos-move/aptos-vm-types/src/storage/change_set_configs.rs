// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::ChangeSetInterface, storage::space_pricing::DiskSpacePricing};
use aptos_gas_algebra::Fee;
use aptos_gas_schedule::{AptosGasParameters, TransactionGasParameters};
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::vm_status::{StatusCode, VMStatus};

#[derive(Clone, Debug)]
pub struct ChangeSetConfigs {
    gas_feature_version: u64,
    pub max_bytes_per_write_op: u64,
    pub max_bytes_all_write_ops_per_transaction: u64,
    pub max_bytes_per_event: u64,
    pub max_bytes_all_events_per_transaction: u64,
    pub max_write_ops_per_transaction: u64,
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

    pub fn check_change_set(&self, change_set: &impl ChangeSetInterface) -> Result<(), VMStatus> {
        if self.max_write_ops_per_transaction != 0
            && change_set.num_write_ops() as u64 > self.max_write_ops_per_transaction
        {
            return storage_write_limit_reached_err(Some("Too many write ops."))
                .map_err(|err| err.into_vm_status());
        }

        let mut tracker = ChangeSetSizeTracker::new(self, None, None);
        for (key, op_size) in change_set.write_set_size_iter() {
            tracker.record_write_op(key, op_size)?;
        }
        for event in change_set.events_iter() {
            tracker.record_event(event)?;
        }

        Ok(())
    }
}

fn storage_write_limit_reached_err(maybe_msg: Option<&str>) -> VMResult<()> {
    let mut err = PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED);
    if let Some(message) = maybe_msg {
        err = err.with_message(message.to_string())
    }
    Err(err.finish(Location::Undefined))
}

pub struct ChangeSetSizeTracker<'a> {
    pub write_fee: Fee,
    pub total_refund: Fee,
    num_write_ops: u64,
    write_set_size: u64,
    total_event_size: u64,
    configs: &'a ChangeSetConfigs,
    pub disk_pricing: Option<&'a DiskSpacePricing>,
    pub txn_gas_params: Option<&'a TransactionGasParameters>,
}

impl<'a> ChangeSetSizeTracker<'a> {
    pub fn new(
        configs: &'a ChangeSetConfigs,
        disk_pricing: Option<&'a DiskSpacePricing>,
        txn_gas_params: Option<&'a TransactionGasParameters>,
    ) -> Self {
        Self {
            write_fee: Fee::new(0),
            total_refund: Fee::new(0),
            num_write_ops: 0,
            write_set_size: 0,
            total_event_size: 0,
            configs,
            disk_pricing,
            txn_gas_params,
        }
    }

    pub fn record_event(&mut self, event: &ContractEvent) -> VMResult<()> {
        let size = event.event_data().len() as u64;
        if size > self.configs.max_bytes_per_event {
            return storage_write_limit_reached_err(None);
        }
        self.total_event_size += size;
        if self.total_event_size > self.configs.max_bytes_all_events_per_transaction {
            return storage_write_limit_reached_err(None);
        }
        Ok(())
    }

    pub fn record_write_op(&mut self, key: &StateKey, write_op_size: WriteOpSize) -> VMResult<()> {
        self.num_write_ops += 1;
        if self.num_write_ops > self.configs.max_write_ops_per_transaction {
            return storage_write_limit_reached_err(Some("Too many write ops."));
        }

        if let Some(len) = write_op_size.write_len() {
            let write_op_size = len + (key.size() as u64);
            if write_op_size > self.configs.max_bytes_per_write_op {
                return storage_write_limit_reached_err(None);
            }
            self.write_set_size += write_op_size;
        }
        if self.write_set_size > self.configs.max_bytes_all_write_ops_per_transaction {
            return storage_write_limit_reached_err(None);
        }
        Ok(())
    }
}
