// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::algebra::{
    AbstractValueSize, FeePerByte, FeePerGasUnit, FeePerSlot, Gas, GasScalingFactor, GasUnit,
    NumBasePoints, NumMicroseconds,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumBytes,
    ToUnitFractionalWithParams, ToUnitWithParams,
};

mod storage;

pub use storage::{ChangeSetConfigs, StorageGasParameters};

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
            2_000_000
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
            10_000_000_000
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
        [memory_quota: AbstractValueSize, { 1.. => "memory_quota" }, 10_000_000],
        [
            free_write_bytes_quota: NumBytes,
            { 5.. => "free_write_bytes_quota" },
            1024, // 1KB free per state write
        ],
        [
            max_bytes_per_write_op: NumBytes,
            { 5.. => "max_bytes_per_write_op" },
            1 << 20, // a single state item is 1MB max
        ],
        [
            max_bytes_all_write_ops_per_transaction: NumBytes,
            { 5.. => "max_bytes_all_write_ops_per_transaction" },
            10 << 20, // all write ops from a single transaction are 10MB max
        ],
        [
            max_bytes_per_event: NumBytes,
            { 5.. => "max_bytes_per_event" },
            1 << 20, // a single event is 1MB max
        ],
        [
            max_bytes_all_events_per_transaction: NumBytes,
            { 5.. => "max_bytes_all_events_per_transaction"},
            10 << 20, // all events from a single transaction are 10MB max
        ],
        [
            per_storage_slot_deposit: FeePerSlot,
            { 7.. => "per_storage_slot_deposit"},
            50_000, // 50k Octas each slot allocation, that's 500k APT for 1 billion slots
        ],
        [
            per_storage_excess_byte_penalty: FeePerByte,
            { 7.. => "per_storage_excess_byte_penalty"},
            // If a storage write is larger than `free_write_bytes_quota`, each excess byte is
            // 50 Octas. It's charged for both creation and modification.
            // As a result, to allocate 1TB of state space by creating large slots, the cost is
            // at least 500k APT.
            50,
        ],
        [
            max_storage_slot_refund_ratio: NumBasePoints,
            { 7.. => "max_storage_slot_refund_ratio"},
            90_00, // if deleted quickly, refund 90% of the deposit
        ],
        [
            min_storage_slot_refund_ratio: NumBasePoints,
            { 7.. => "max_storage_slot_refund_ratio"},
            50_00, // deleting a fairly old item still yields 50% refund
        ],
        [
            storage_slot_refund_degrade_start: NumMicroseconds,
            { 7.. => "storage_slot_refund_degrade_start"},
            86_400_000_000, // maximum refund if slot is freed within 24 hours
        ],
        [
            storage_slot_refund_degrade_period: NumMicroseconds,
            { 7.. => "storage_slot_refund_degrade_period"},
            // Minimal refund if a slot if freed later than 31 days. That is "1 day" from the
            // previous parameter plus "30 days", the value of this parameter.
            2_592_000_000_000,
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
