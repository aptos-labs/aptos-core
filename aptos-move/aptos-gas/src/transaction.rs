// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::algebra::{FeePerGasUnit, Gas, GasScalingFactor, GasUnit};
use aptos_types::{
    on_chain_config::StorageGasSchedule, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumArgs, NumBytes,
    ToUnitFractionalWithParams, ToUnitWithParams,
};

#[derive(Clone, Debug)]
pub struct StorageGasParameters {
    pub per_item_read: InternalGasPerArg,
    pub per_item_create: InternalGasPerArg,
    pub per_item_write: InternalGasPerArg,
    pub per_byte_read: InternalGasPerByte,
    pub per_byte_create: InternalGasPerByte,
    pub per_byte_write: InternalGasPerByte,
}

impl From<StorageGasSchedule> for StorageGasParameters {
    fn from(gas_schedule: StorageGasSchedule) -> Self {
        Self {
            per_item_read: gas_schedule.per_item_read.into(),
            per_item_create: gas_schedule.per_item_create.into(),
            per_item_write: gas_schedule.per_item_write.into(),
            per_byte_read: gas_schedule.per_byte_read.into(),
            per_byte_create: gas_schedule.per_byte_create.into(),
            per_byte_write: gas_schedule.per_byte_write.into(),
        }
    }
}

impl StorageGasParameters {
    pub fn zeros() -> Self {
        Self {
            per_item_read: 0.into(),
            per_item_create: 0.into(),
            per_item_write: 0.into(),
            per_byte_read: 0.into(),
            per_byte_create: 0.into(),
            per_byte_write: 0.into(),
        }
    }
}

impl StorageGasParameters {
    pub fn calculate_write_set_gas<'a>(
        &self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> InternalGas {
        use WriteOp::*;

        let mut num_items_create = NumArgs::zero();
        let mut num_items_write = NumArgs::zero();
        let mut num_bytes_create = NumBytes::zero();
        let mut num_bytes_write = NumBytes::zero();

        for (key, op) in ops.into_iter() {
            let key_size = || {
                NumBytes::new(
                    key.encode()
                        .expect("Should be able to serialize state key")
                        .len() as u64,
                )
            };

            match op {
                Creation(data) => {
                    num_items_create += 1.into();

                    num_bytes_create += key_size();
                    num_bytes_create += NumBytes::new(data.len() as u64);
                }
                Modification(data) => {
                    num_items_write += 1.into();

                    num_bytes_write += key_size();
                    num_bytes_write += NumBytes::new(data.len() as u64);
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

crate::params::define_gas_parameters!(
    TransactionGasParameters,
    "txn",
    [
        // The flat minimum amount of gas required for any transaction.
        // Charged at the start of execution.
        [
            min_transaction_gas_units: InternalGas,
            "min_transaction_gas_units",
            1_500_000
        ],
        // Any transaction over this size will be charged an additional amount per byte.
        [
            large_transaction_cutoff: NumBytes,
            "large_transaction_cutoff",
            600
        ],
        // The units of gas that to be charged per byte over the `large_transaction_cutoff` in addition to
        // `min_transaction_gas_units` for transactions whose size exceeds `large_transaction_cutoff`.
        [
            intrinsic_gas_per_byte: InternalGasPerByte,
            "intrinsic_gas_per_byte",
            2_000
        ],
        // ~5 microseconds should equal one unit of computational gas. We bound the maximum
        // computational time of any given transaction at roughly 20 seconds. We want this number and
        // `MAX_PRICE_PER_GAS_UNIT` to always satisfy the inequality that
        // MAXIMUM_NUMBER_OF_GAS_UNITS * MAX_PRICE_PER_GAS_UNIT < min(u64::MAX, GasUnits<GasCarrier>::MAX)
        [
            maximum_number_of_gas_units: Gas,
            "maximum_number_of_gas_units",
            10_000_000
        ],
        // The minimum gas price that a transaction can be submitted with.
        // TODO(Gas): should probably change this to something > 0
        [
            min_price_per_gas_unit: FeePerGasUnit,
            "min_price_per_gas_unit",
            aptos_global_constants::GAS_UNIT_PRICE
        ],
        // The maximum gas unit price that a transaction can be submitted with.
        [
            max_price_per_gas_unit: FeePerGasUnit,
            "max_price_per_gas_unit",
            10_000
        ],
        [
            max_transaction_size_in_bytes: NumBytes,
            "max_transaction_size_in_bytes",
            64 * 1024
        ],
        [
            gas_unit_scaling_factor: GasScalingFactor,
            "gas_unit_scaling_factor",
            10_000
        ],
        // Gas Parameters for reading data from storage.
        [load_data_base: InternalGas, "load_data.base", 16_000],
        [
            load_data_per_byte: InternalGasPerByte,
            "load_data.per_byte",
            1_000
        ],
        [load_data_failure: InternalGas, "load_data.failure", 0],
        // Gas parameters for writing data to storage.
        [
            write_data_per_op: InternalGasPerArg,
            "write_data.per_op",
            160_000
        ],
        [
            write_data_per_new_item: InternalGasPerArg,
            "write_data.new_item",
            1_280_000
        ],
        [
            write_data_per_byte_in_key: InternalGasPerByte,
            "write_data.per_byte_in_key",
            10_000
        ],
        [
            write_data_per_byte_in_val: InternalGasPerByte,
            "write_data.per_byte_in_val",
            10_000
        ],
    ]
);

impl TransactionGasParameters {
    // TODO(Gas): Right now we are relying on this to avoid div by zero errors when using the all-zero
    //            gas parameters. See if there's a better way we can handle this.
    fn scaling_factor(&self) -> GasScalingFactor {
        match u64::from(self.gas_unit_scaling_factor) {
            0 => 1.into(),
            x => x.into(),
        }
    }

    /// Calculate the intrinsic gas for the transaction based upon its size in bytes.
    pub fn calculate_intrinsic_gas(&self, transaction_size: NumBytes) -> InternalGas {
        let min_transaction_fee = self.min_transaction_gas_units;

        if transaction_size > self.large_transaction_cutoff {
            let excess = transaction_size
                .checked_sub(self.large_transaction_cutoff)
                .unwrap();
            min_transaction_fee + (excess * self.intrinsic_gas_per_byte)
        } else {
            min_transaction_fee
        }
    }

    pub fn calculate_write_set_gas<'a>(
        &self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
    ) -> InternalGas {
        use WriteOp::*;

        // Counting
        let mut num_ops = NumArgs::zero();
        let mut num_new_items = NumArgs::zero();
        let mut num_bytes_key = NumBytes::zero();
        let mut num_bytes_val = NumBytes::zero();

        for (key, op) in ops.into_iter() {
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

impl ToUnitWithParams<InternalGasUnit> for GasUnit {
    type Params = TransactionGasParameters;

    fn multiplier(params: &Self::Params) -> u64 {
        params.scaling_factor().into()
    }
}

impl ToUnitFractionalWithParams<GasUnit> for InternalGasUnit {
    type Params = TransactionGasParameters;

    fn ratio(params: &Self::Params) -> (u64, u64) {
        (1, params.scaling_factor().into())
    }
}
