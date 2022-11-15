// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::transaction::ChangeSetLimits;
use aptos_types::{
    on_chain_config::StorageGasSchedule, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes,
};
use std::{fmt::Debug, sync::Arc};

pub trait StoragePricingTrait: Debug + Send + Sync {
    fn calculate_read_gas(&self, _loaded: Option<NumBytes>) -> InternalGas;

    fn calculate_write_set_gas<'a>(
        &self,
        _ops: &mut dyn Iterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> InternalGas;
}

#[derive(Debug)]
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
            write_data_per_op: gas_params.txn.write_data_per_op,
            write_data_per_new_item: gas_params.txn.write_data_per_new_item,
            write_data_per_byte_in_key: gas_params.txn.write_data_per_byte_in_key,
            write_data_per_byte_in_val: gas_params.txn.write_data_per_byte_in_val,
            load_data_base: gas_params.txn.load_data_base,
            load_data_per_byte: gas_params.txn.load_data_per_byte,
            load_data_failure: gas_params.txn.load_data_failure,
        }
    }
}

impl StoragePricingTrait for StoragePricingV1 {
    fn calculate_read_gas(&self, loaded: Option<NumBytes>) -> InternalGas {
        self.load_data_base
            + match loaded {
                Some(num_bytes) => self.load_data_per_byte * num_bytes,
                None => self.load_data_failure,
            }
    }

    fn calculate_write_set_gas<'a>(
        &self,
        ops: &mut dyn Iterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> InternalGas {
        use WriteOp::*;

        // Counting
        let mut num_ops = NumArgs::zero();
        let mut num_new_items = NumArgs::zero();
        let mut num_bytes_key = NumBytes::zero();
        let mut num_bytes_val = NumBytes::zero();

        for (key, op) in ops {
            num_ops += 1.into();

            if self.write_data_per_byte_in_key > 0.into() {
                // TODO(Gas): Are we supposed to panic here?
                num_bytes_key += NumBytes::new(
                    key.encode()
                        .expect("Should be able to serialize state key")
                        .len() as u64,
                );
            }

            match op {
                Creation(data) => {
                    num_new_items += 1.into();
                    num_bytes_val += NumBytes::new(data.len() as u64);
                }
                Modification(data) => {
                    num_bytes_val += NumBytes::new(data.len() as u64);
                }
                Deletion => (),
            }
        }

        // Calculate the costs
        let cost_ops = self.write_data_per_op * num_ops;
        let cost_new_items = self.write_data_per_new_item * num_new_items;
        let cost_bytes = self.write_data_per_byte_in_key * num_bytes_key
            + self.write_data_per_byte_in_val * num_bytes_val;

        cost_ops + cost_new_items + cost_bytes
    }
}

#[derive(Debug)]
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
    pub fn zeros() -> Self {
        Self::new(
            LATEST_GAS_FEATURE_VERSION,
            &StorageGasSchedule::zeros(),
            &AptosGasParameters::zeros(),
        )
    }

    pub fn new(
        feature_version: u64,
        storage_gas_schedule: &StorageGasSchedule,
        gas_params: &AptosGasParameters,
    ) -> Self {
        assert!(feature_version > 0);

        let free_write_bytes_quota = if feature_version >= 5 {
            gas_params.txn.free_write_bytes_quota
        } else if feature_version >= 3 {
            1024.into()
        } else {
            // for feature_version 2 and below `free_write_bytes_quota` won't be used anyway
            // but let's set it properly to reduce confusion.
            0.into()
        };

        Self {
            feature_version,
            free_write_bytes_quota,
            per_item_read: storage_gas_schedule.per_item_read.into(),
            per_item_create: storage_gas_schedule.per_item_create.into(),
            per_item_write: storage_gas_schedule.per_item_write.into(),
            per_byte_read: storage_gas_schedule.per_byte_read.into(),
            per_byte_create: storage_gas_schedule.per_byte_create.into(),
            per_byte_write: storage_gas_schedule.per_byte_write.into(),
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
}

impl StoragePricingTrait for StoragePricingV2 {
    fn calculate_read_gas(&self, loaded: Option<NumBytes>) -> InternalGas {
        self.per_item_read * (NumArgs::from(1))
            + match loaded {
                Some(num_bytes) => self.per_byte_read * num_bytes,
                None => 0.into(),
            }
    }

    fn calculate_write_set_gas<'a>(
        &self,
        ops: &mut dyn Iterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> InternalGas {
        use aptos_types::write_set::WriteOp::*;

        let mut num_items_create = NumArgs::zero();
        let mut num_items_write = NumArgs::zero();
        let mut num_bytes_create = NumBytes::zero();
        let mut num_bytes_write = NumBytes::zero();

        for (key, op) in ops {
            match &op {
                Creation(data) => {
                    num_items_create += 1.into();
                    num_bytes_create += self.write_op_size(key, data);
                }
                Modification(data) => {
                    num_items_write += 1.into();
                    num_bytes_write += self.write_op_size(key, data);
                }
                Deletion => (),
            }
        }

        num_items_create * self.per_item_create
            + num_items_write * self.per_item_write
            + num_bytes_create * self.per_byte_create
            + num_bytes_write * self.per_byte_write
    }
}

struct ChangeSetLimitsBuilder;

impl ChangeSetLimitsBuilder {
    pub fn build(feature_version: u64, gas_params: &AptosGasParameters) -> ChangeSetLimits {
        if feature_version >= 5 {
            Self::from_gas_params(feature_version, gas_params)
        } else if feature_version >= 3 {
            Self::for_feature_version_3()
        } else {
            ChangeSetLimits::unlimited_at_gas_feature_version(feature_version)
        }
    }

    fn for_feature_version_3() -> ChangeSetLimits {
        const MB: u64 = 1 << 20;

        ChangeSetLimits::new(3, MB, u64::MAX, MB, MB << 10)
    }

    fn from_gas_params(
        gas_feature_version: u64,
        gas_params: &AptosGasParameters,
    ) -> ChangeSetLimits {
        ChangeSetLimits::new(
            gas_feature_version,
            gas_params.txn.max_bytes_per_write_op.into(),
            gas_params
                .txn
                .max_bytes_all_write_ops_per_transaction
                .into(),
            gas_params.txn.max_bytes_per_event.into(),
            gas_params.txn.max_bytes_all_events_per_transaction.into(),
        )
    }
}

#[derive(Clone)]
pub struct StorageGasParameters {
    pub pricing: Arc<dyn StoragePricingTrait>,
    pub limits: ChangeSetLimits,
}

impl StorageGasParameters {
    pub fn new(
        feature_version: u64,
        gas_params: Option<&AptosGasParameters>,
        storage_gas_schedule: Option<&StorageGasSchedule>,
    ) -> Option<Self> {
        if feature_version == 0 || gas_params.is_none() {
            return None;
        }
        let gas_params = gas_params.unwrap();

        let pricing: Arc<dyn StoragePricingTrait> = match storage_gas_schedule {
            Some(schedule) => {
                Arc::new(StoragePricingV2::new(feature_version, schedule, gas_params))
            }
            None => Arc::new(StoragePricingV1::new(gas_params)),
        };

        let limits = ChangeSetLimitsBuilder::build(feature_version, gas_params);

        Some(Self { pricing, limits })
    }

    pub fn free_and_unlimited() -> Self {
        Self {
            pricing: Arc::new(StoragePricingV2::zeros()),
            limits: ChangeSetLimits::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION),
        }
    }
}
