// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::GasExpression;
use aptos_gas_schedule::{
    gas_params::txn::{
        STORAGE_IO_PER_EVENT_BYTE_WRITE, STORAGE_IO_PER_STATE_BYTE_READ,
        STORAGE_IO_PER_STATE_BYTE_WRITE, STORAGE_IO_PER_STATE_SLOT_READ,
        STORAGE_IO_PER_STATE_SLOT_WRITE, STORAGE_IO_PER_TRANSACTION_BYTE_WRITE,
    },
    AptosGasParameters, VMGasParameters,
};
use aptos_types::{
    contract_event::ContractEvent,
    on_chain_config::{ConfigStorage, StorageGasSchedule},
    state_store::state_key::StateKey,
    write_set::WriteOpSize,
};
use either::Either;
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumArgs, NumBytes,
};

#[derive(Clone, Debug)]
pub struct IoPricingV1 {
    write_data_per_op: InternalGasPerArg,
    write_data_per_new_item: InternalGasPerArg,
    write_data_per_byte_in_key: InternalGasPerByte,
    write_data_per_byte_in_val: InternalGasPerByte,
    load_data_base: InternalGas,
    load_data_per_byte: InternalGasPerByte,
    load_data_failure: InternalGas,
}

impl IoPricingV1 {
    fn new(gas_params: &AptosGasParameters) -> Self {
        Self {
            write_data_per_op: gas_params.vm.txn.storage_io_per_state_slot_write,
            write_data_per_new_item: gas_params.vm.txn.legacy_write_data_per_new_item,
            write_data_per_byte_in_key: gas_params.vm.txn.storage_io_per_state_byte_write,
            write_data_per_byte_in_val: gas_params.vm.txn.legacy_write_data_per_byte_in_val,
            load_data_base: gas_params.vm.txn.storage_io_per_state_slot_read * NumArgs::new(1),
            load_data_per_byte: gas_params.vm.txn.storage_io_per_state_byte_read,
            load_data_failure: gas_params.vm.txn.load_data_failure,
        }
    }
}

impl IoPricingV1 {
    fn calculate_read_gas(&self, loaded: Option<NumBytes>) -> InternalGas {
        self.load_data_base
            + match loaded {
                Some(num_bytes) => self.load_data_per_byte * num_bytes,
                None => self.load_data_failure,
            }
    }

    fn io_gas_per_write(&self, key: &StateKey, op_size: &WriteOpSize) -> InternalGas {
        use aptos_types::write_set::WriteOpSize::*;

        let mut cost = self.write_data_per_op * NumArgs::new(1);

        if self.write_data_per_byte_in_key > 0.into() {
            cost += self.write_data_per_byte_in_key * NumBytes::new(key.encoded().len() as u64);
        }

        match op_size {
            Creation { write_len } => {
                cost += self.write_data_per_new_item * NumArgs::new(1)
                    + self.write_data_per_byte_in_val * NumBytes::new(*write_len);
            },
            Modification { write_len } => {
                cost += self.write_data_per_byte_in_val * NumBytes::new(*write_len);
            },
            Deletion => (),
        }

        cost
    }
}

#[derive(Clone, Debug)]
pub struct IoPricingV2 {
    pub feature_version: u64,
    pub free_write_bytes_quota: NumBytes,
    pub per_item_read: InternalGasPerArg,
    pub per_item_create: InternalGasPerArg,
    pub per_item_write: InternalGasPerArg,
    pub per_byte_read: InternalGasPerByte,
    pub per_byte_create: InternalGasPerByte,
    pub per_byte_write: InternalGasPerByte,
}

impl IoPricingV2 {
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
            5.. => gas_params.vm.txn.legacy_free_write_bytes_quota,
        }
    }

    fn write_op_size(&self, key: &StateKey, value_size: u64) -> NumBytes {
        let value_size = NumBytes::new(value_size);

        if self.feature_version >= 3 {
            let key_size = NumBytes::new(key.size() as u64);
            (key_size + value_size)
                .checked_sub(self.free_write_bytes_quota)
                .unwrap_or(NumBytes::zero())
        } else {
            let key_size = NumBytes::new(key.encoded().len() as u64);
            key_size + value_size
        }
    }

    fn calculate_read_gas(&self, loaded: NumBytes) -> InternalGas {
        self.per_item_read * (NumArgs::from(1)) + self.per_byte_read * loaded
    }

    fn io_gas_per_write(&self, key: &StateKey, op_size: &WriteOpSize) -> InternalGas {
        use aptos_types::write_set::WriteOpSize::*;

        match op_size {
            Creation { write_len } => {
                self.per_item_create * NumArgs::new(1)
                    + self.write_op_size(key, *write_len) * self.per_byte_create
            },
            Modification { write_len } => {
                self.per_item_write * NumArgs::new(1)
                    + self.write_op_size(key, *write_len) * self.per_byte_write
            },
            Deletion => 0.into(),
        }
    }
}

// No storage curve. New gas parameter representation.
#[derive(Debug, Clone)]
pub struct IoPricingV3 {
    pub feature_version: u64,
    pub legacy_free_write_bytes_quota: NumBytes,
}

impl IoPricingV3 {
    fn calculate_read_gas(
        &self,
        loaded: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        STORAGE_IO_PER_STATE_SLOT_READ * NumArgs::from(1) + STORAGE_IO_PER_STATE_BYTE_READ * loaded
    }

    fn write_op_size(&self, key: &StateKey, value_size: u64) -> NumBytes {
        let key_size = NumBytes::new(key.size() as u64);
        let value_size = NumBytes::new(value_size);

        (key_size + value_size)
            .checked_sub(self.legacy_free_write_bytes_quota)
            .unwrap_or(NumBytes::zero())
    }

    fn io_gas_per_write(
        &self,
        key: &StateKey,
        op_size: &WriteOpSize,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        op_size.write_len().map_or_else(
            || Either::Right(InternalGas::zero()),
            |write_len| {
                Either::Left(
                    STORAGE_IO_PER_STATE_SLOT_WRITE * NumArgs::new(1)
                        + STORAGE_IO_PER_STATE_BYTE_WRITE * self.write_op_size(key, write_len),
                )
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct IoPricingV4;

impl IoPricingV4 {
    fn calculate_read_gas(
        &self,
        loaded: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        // Round up bytes to whole pages
        // TODO(gas): make PAGE_SIZE configurable
        const PAGE_SIZE: u64 = 4096;

        let loaded_u64: u64 = loaded.into();
        let r = loaded_u64 % PAGE_SIZE;
        let rounded_up = loaded_u64 + if r == 0 { 0 } else { PAGE_SIZE - r };

        STORAGE_IO_PER_STATE_SLOT_READ * NumArgs::from(1)
            + STORAGE_IO_PER_STATE_BYTE_READ * NumBytes::new(rounded_up)
    }

    fn io_gas_per_write(
        &self,
        key: &StateKey,
        op_size: &WriteOpSize,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        let key_size = NumBytes::new(key.size() as u64);
        let value_size = NumBytes::new(op_size.write_len().unwrap_or(0));
        let size = key_size + value_size;

        STORAGE_IO_PER_STATE_SLOT_WRITE * NumArgs::new(1) + STORAGE_IO_PER_STATE_BYTE_WRITE * size
    }
}

#[derive(Clone, Debug)]
pub enum IoPricing {
    V1(IoPricingV1),
    V2(IoPricingV2),
    V3(IoPricingV3),
    V4(IoPricingV4),
}

impl IoPricing {
    pub fn new(
        feature_version: u64,
        gas_params: &AptosGasParameters,
        config_storage: &impl ConfigStorage,
    ) -> IoPricing {
        use aptos_types::on_chain_config::OnChainConfig;
        use IoPricing::*;

        match feature_version {
            0 => V1(IoPricingV1::new(gas_params)),
            1..=9 => match StorageGasSchedule::fetch_config(config_storage) {
                None => V1(IoPricingV1::new(gas_params)),
                Some(schedule) => V2(IoPricingV2::new_with_storage_curves(
                    feature_version,
                    &schedule,
                    gas_params,
                )),
            },
            10..=11 => V3(IoPricingV3 {
                feature_version,
                legacy_free_write_bytes_quota: gas_params.vm.txn.legacy_free_write_bytes_quota,
            }),
            12.. => V4(IoPricingV4),
        }
    }

    pub fn calculate_read_gas(
        &self,
        resource_exists: bool,
        bytes_loaded: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        use IoPricing::*;

        match self {
            V1(v1) => Either::Left(v1.calculate_read_gas(
                if resource_exists {
                    Some(bytes_loaded)
                } else {
                    None
                },
            )),
            V2(v2) => Either::Left(v2.calculate_read_gas(bytes_loaded)),
            V3(v3) => Either::Right(Either::Left(v3.calculate_read_gas(bytes_loaded))),
            V4(v4) => Either::Right(Either::Right(v4.calculate_read_gas(bytes_loaded))),
        }
    }

    pub fn io_gas_per_transaction(
        &self,
        txn_size: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        STORAGE_IO_PER_TRANSACTION_BYTE_WRITE * txn_size
    }

    pub fn io_gas_per_event(
        &self,
        event: &ContractEvent,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        STORAGE_IO_PER_EVENT_BYTE_WRITE * NumBytes::new(event.size() as u64)
    }

    /// If group write size is provided, then the StateKey is for a resource group and the
    /// WriteOp does not contain the raw data, and the provided size should be used instead.
    pub fn io_gas_per_write(
        &self,
        key: &StateKey,
        op_size: &WriteOpSize,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        use IoPricing::*;

        match self {
            V1(v1) => Either::Left(v1.io_gas_per_write(key, op_size)),
            V2(v2) => Either::Left(v2.io_gas_per_write(key, op_size)),
            V3(v3) => Either::Right(Either::Left(v3.io_gas_per_write(key, op_size))),
            V4(v4) => Either::Right(Either::Right(v4.io_gas_per_write(key, op_size))),
        }
    }
}
