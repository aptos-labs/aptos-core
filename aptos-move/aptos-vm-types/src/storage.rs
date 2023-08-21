// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{change_set::VMChangeSet, check_change_set::CheckChangeSet};
use aptos_gas_algebra::GasExpression;
use aptos_gas_schedule::{
    gas_params::txn::*, AptosGasParameters, VMGasParameters, LATEST_GAS_FEATURE_VERSION,
};
use aptos_types::{
    on_chain_config::{ConfigStorage, OnChainConfig, StorageGasSchedule},
    state_store::state_key::StateKey,
    write_set::WriteOp,
};
use either::Either;
use move_core_types::{
    gas_algebra::{
        InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumArgs, NumBytes,
    },
    vm_status::{StatusCode, VMStatus},
};
use std::fmt::Debug;

#[derive(Clone, Debug)]
pub struct StoragePricingV1 {
    write_data_per_op: InternalGasPerArg,
    write_data_per_new_item: InternalGasPerArg,
    write_data_per_byte_in_key: InternalGasPerByte,
    write_data_per_byte_in_val: InternalGasPerByte,
    load_data_base: InternalGas,
    load_data_per_byte: InternalGasPerByte,
    load_data_failure: InternalGas,
}

impl StoragePricingV1 {
    fn new(gas_params: &AptosGasParameters) -> Self {
        Self {
            write_data_per_op: gas_params.vm.txn.storage_io_per_state_slot_write,
            write_data_per_new_item: gas_params.vm.txn.write_data_per_new_item,
            write_data_per_byte_in_key: gas_params.vm.txn.storage_io_per_state_byte_write,
            write_data_per_byte_in_val: gas_params.vm.txn.write_data_per_byte_in_val,
            load_data_base: gas_params.vm.txn.storage_io_per_state_slot_read * NumArgs::new(1),
            load_data_per_byte: gas_params.vm.txn.storage_io_per_state_byte_read,
            load_data_failure: gas_params.vm.txn.load_data_failure,
        }
    }
}

impl StoragePricingV1 {
    fn calculate_read_gas(&self, loaded: Option<NumBytes>) -> InternalGas {
        self.load_data_base
            + match loaded {
                Some(num_bytes) => self.load_data_per_byte * num_bytes,
                None => self.load_data_failure,
            }
    }

    fn io_gas_per_write(&self, key: &StateKey, op: &WriteOp) -> InternalGas {
        use aptos_types::write_set::WriteOp::*;

        let mut cost = self.write_data_per_op * NumArgs::new(1);

        if self.write_data_per_byte_in_key > 0.into() {
            cost += self.write_data_per_byte_in_key
                * NumBytes::new(
                    key.encode()
                        .expect("Should be able to serialize state key")
                        .len() as u64,
                );
        }

        match op {
            Creation(data) | CreationWithMetadata { data, .. } => {
                cost += self.write_data_per_new_item * NumArgs::new(1)
                    + self.write_data_per_byte_in_val * NumBytes::new(data.len() as u64);
            },
            Modification(data) | ModificationWithMetadata { data, .. } => {
                cost += self.write_data_per_byte_in_val * NumBytes::new(data.len() as u64);
            },
            Deletion | DeletionWithMetadata { .. } => (),
        }

        cost
    }
}

#[derive(Clone, Debug)]
pub struct StoragePricingV2 {
    pub feature_version: u64,
    pub free_write_bytes_quota: NumBytes,
    pub per_item_read: InternalGasPerArg,
    pub per_item_create: InternalGasPerArg,
    pub per_item_write: InternalGasPerArg,
    pub per_byte_read: InternalGasPerByte,
    pub per_byte_create: InternalGasPerByte,
    pub per_byte_write: InternalGasPerByte,
}

impl StoragePricingV2 {
    pub fn new_with_storage_curves(
        feature_version: u64,
        storage_gas_schedule: &StorageGasSchedule,
        gas_params: &AptosGasParameters,
    ) -> Self {
        Self {
            feature_version,
            free_write_bytes_quota: Self::get_free_write_bytes_quota(feature_version, gas_params),
            per_item_read: storage_gas_schedule.per_item_read.into(),
            per_item_create: storage_gas_schedule.per_item_create.into(),
            per_item_write: storage_gas_schedule.per_item_write.into(),
            per_byte_read: storage_gas_schedule.per_byte_read.into(),
            per_byte_create: storage_gas_schedule.per_byte_create.into(),
            per_byte_write: storage_gas_schedule.per_byte_write.into(),
        }
    }

    fn get_free_write_bytes_quota(
        feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> NumBytes {
        match feature_version {
            0 => unreachable!("PricingV2 not applicable for feature version 0"),
            1..=2 => 0.into(),
            3..=4 => 1024.into(),
            5.. => gas_params.vm.txn.free_write_bytes_quota,
        }
    }

    fn write_op_size(&self, key: &StateKey, value: &[u8]) -> NumBytes {
        let value_size = NumBytes::new(value.len() as u64);

        if self.feature_version >= 3 {
            let key_size = NumBytes::new(key.size() as u64);
            (key_size + value_size)
                .checked_sub(self.free_write_bytes_quota)
                .unwrap_or(NumBytes::zero())
        } else {
            let key_size = NumBytes::new(
                key.encode()
                    .expect("Should be able to serialize state key")
                    .len() as u64,
            );
            key_size + value_size
        }
    }

    fn calculate_read_gas(&self, loaded: NumBytes) -> InternalGas {
        self.per_item_read * (NumArgs::from(1)) + self.per_byte_read * loaded
    }

    fn io_gas_per_write(&self, key: &StateKey, op: &WriteOp) -> InternalGas {
        use aptos_types::write_set::WriteOp::*;

        match &op {
            Creation(data) | CreationWithMetadata { data, .. } => {
                self.per_item_create * NumArgs::new(1)
                    + self.write_op_size(key, data) * self.per_byte_create
            },
            Modification(data) | ModificationWithMetadata { data, .. } => {
                self.per_item_write * NumArgs::new(1)
                    + self.write_op_size(key, data) * self.per_byte_write
            },
            Deletion | DeletionWithMetadata { .. } => 0.into(),
        }
    }
}

// No storage curve. New gas parameter representation.
#[derive(Debug, Clone)]
pub struct StoragePricingV3 {
    pub feature_version: u64,
    pub free_write_bytes_quota: NumBytes,
}

impl StoragePricingV3 {
    fn calculate_read_gas(
        &self,
        loaded: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> {
        STORAGE_IO_PER_STATE_SLOT_READ * NumArgs::from(1) + STORAGE_IO_PER_STATE_BYTE_READ * loaded
    }

    fn write_op_size(&self, key: &StateKey, value: &[u8]) -> NumBytes {
        let value_size = NumBytes::new(value.len() as u64);
        let key_size = NumBytes::new(key.size() as u64);

        (key_size + value_size)
            .checked_sub(self.free_write_bytes_quota)
            .unwrap_or(NumBytes::zero())
    }

    fn io_gas_per_write(
        &self,
        key: &StateKey,
        op: &WriteOp,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> {
        use WriteOp::*;

        match op {
            Creation(data)
            | CreationWithMetadata { data, .. }
            | Modification(data)
            | ModificationWithMetadata { data, .. } => Either::Left(
                STORAGE_IO_PER_STATE_SLOT_WRITE * NumArgs::new(1)
                    + STORAGE_IO_PER_STATE_BYTE_WRITE * self.write_op_size(key, data),
            ),
            Deletion | DeletionWithMetadata { .. } => Either::Right(InternalGas::zero()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum StoragePricing {
    V1(StoragePricingV1),
    V2(StoragePricingV2),
    V3(StoragePricingV3),
}

impl StoragePricing {
    pub fn new(
        feature_version: u64,
        gas_params: &AptosGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> StoragePricing {
        use StoragePricing::*;

        match feature_version {
            0 => V1(StoragePricingV1::new(gas_params)),
            1..=9 => match StorageGasSchedule::fetch_config(config_storage) {
                None => V1(StoragePricingV1::new(gas_params)),
                Some(schedule) => V2(StoragePricingV2::new_with_storage_curves(
                    feature_version,
                    &schedule,
                    gas_params,
                )),
            },
            10.. => V3(StoragePricingV3 {
                feature_version,
                free_write_bytes_quota: gas_params.vm.txn.free_write_bytes_quota,
            }),
        }
    }

    pub fn calculate_read_gas(
        &self,
        resource_exists: bool,
        bytes_loaded: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> {
        use StoragePricing::*;

        match self {
            V1(v1) => Either::Left(v1.calculate_read_gas(
                if resource_exists {
                    Some(bytes_loaded)
                } else {
                    None
                },
            )),
            V2(v2) => Either::Left(v2.calculate_read_gas(bytes_loaded)),
            V3(v3) => Either::Right(v3.calculate_read_gas(bytes_loaded)),
        }
    }

    pub fn io_gas_per_write(
        &self,
        key: &StateKey,
        op: &WriteOp,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> {
        use StoragePricing::*;

        match self {
            V1(v1) => Either::Left(v1.io_gas_per_write(key, op)),
            V2(v2) => Either::Left(v2.io_gas_per_write(key, op)),
            V3(v3) => Either::Right(v3.io_gas_per_write(key, op)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChangeSetConfigs {
    gas_feature_version: u64,
    max_bytes_per_write_op: u64,
    max_bytes_all_write_ops_per_transaction: u64,
    max_bytes_per_event: u64,
    max_bytes_all_events_per_transaction: u64,
}

impl ChangeSetConfigs {
    pub fn unlimited_at_gas_feature_version(gas_feature_version: u64) -> Self {
        Self::new_impl(gas_feature_version, u64::MAX, u64::MAX, u64::MAX, u64::MAX)
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
    ) -> Self {
        Self {
            gas_feature_version,
            max_bytes_per_write_op,
            max_bytes_all_write_ops_per_transaction,
            max_bytes_per_event,
            max_bytes_all_events_per_transaction,
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

        Self::new_impl(3, MB, u64::MAX, MB, 10 * MB)
    }

    fn from_gas_params(gas_feature_version: u64, gas_params: &AptosGasParameters) -> Self {
        Self::new_impl(
            gas_feature_version,
            gas_params.vm.txn.max_bytes_per_write_op.into(),
            gas_params
                .vm
                .txn
                .max_bytes_all_write_ops_per_transaction
                .into(),
            gas_params.vm.txn.max_bytes_per_event.into(),
            gas_params
                .vm
                .txn
                .max_bytes_all_events_per_transaction
                .into(),
        )
    }
}

impl CheckChangeSet for ChangeSetConfigs {
    fn check_change_set(&self, change_set: &VMChangeSet) -> Result<(), VMStatus> {
        const ERR: StatusCode = StatusCode::STORAGE_WRITE_LIMIT_REACHED;

        let mut write_set_size = 0;
        for (key, op) in change_set.write_set_iter() {
            if let Some(bytes) = op.bytes() {
                let write_op_size = (bytes.len() + key.size()) as u64;
                if write_op_size > self.max_bytes_per_write_op {
                    return Err(VMStatus::error(ERR, None));
                }
                write_set_size += write_op_size;
            }
            if write_set_size > self.max_bytes_all_write_ops_per_transaction {
                return Err(VMStatus::error(ERR, None));
            }
        }

        let mut total_event_size = 0;
        for event in change_set.events() {
            let size = event.event_data().len() as u64;
            if size > self.max_bytes_per_event {
                return Err(VMStatus::error(ERR, None));
            }
            total_event_size += size;
            if total_event_size > self.max_bytes_all_events_per_transaction {
                return Err(VMStatus::error(ERR, None));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct StorageGasParameters {
    pub pricing: StoragePricing,
    pub change_set_configs: ChangeSetConfigs,
}

impl StorageGasParameters {
    pub fn new(
        feature_version: u64,
        gas_params: &AptosGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> Self {
        let pricing = StoragePricing::new(feature_version, gas_params, config_storage);
        let change_set_configs = ChangeSetConfigs::new(feature_version, gas_params);

        Self {
            pricing,
            change_set_configs,
        }
    }

    pub fn unlimited(free_write_bytes_quota: NumBytes) -> Self {
        Self {
            pricing: StoragePricing::V3(StoragePricingV3 {
                feature_version: LATEST_GAS_FEATURE_VERSION,
                free_write_bytes_quota,
            }),
            change_set_configs: ChangeSetConfigs::unlimited_at_gas_feature_version(
                LATEST_GAS_FEATURE_VERSION,
            ),
        }
    }
}
