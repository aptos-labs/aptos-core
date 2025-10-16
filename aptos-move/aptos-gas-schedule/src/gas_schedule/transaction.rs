// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::{
    gas_schedule::VMGasParameters,
    ver::gas_feature_versions::{
        RELEASE_V1_10, RELEASE_V1_11, RELEASE_V1_12, RELEASE_V1_13, RELEASE_V1_15, RELEASE_V1_26,
    },
};
use aptos_gas_algebra::{
    AbstractValueSize, Fee, FeePerByte, FeePerGasUnit, FeePerSlot, Gas, GasExpression,
    GasScalingFactor, GasUnit, NumModules, NumSlots, NumTypeNodes,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumBytes, ToUnitWithParams,
};

const GAS_SCALING_FACTOR: u64 = 1_000_000;

crate::gas_schedule::macros::define_gas_parameters!(
    TransactionGasParameters,
    "txn",
    VMGasParameters => .txn,
    [
        // The flat minimum amount of gas required for any transaction.
        // Charged at the start of execution.
        // It is variable to charge more for more expensive authenticators, e.g., keyless
        [
            min_transaction_gas_units: InternalGas,
            "min_transaction_gas_units",
            2_760_000
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
            1_158
        ],
        // ~5 microseconds should equal one unit of computational gas. We bound the maximum
        // computational time of any given transaction at roughly 20 seconds. We want this number and
        // `MAX_PRICE_PER_GAS_UNIT` to always satisfy the inequality that
        // MAXIMUM_NUMBER_OF_GAS_UNITS * MAX_PRICE_PER_GAS_UNIT < min(u64::MAX, GasUnits<GasCarrier>::MAX)
        [
            maximum_number_of_gas_units: Gas,
            "maximum_number_of_gas_units",
            aptos_global_constants::MAX_GAS_AMOUNT
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
            max_transaction_size_in_bytes_gov: NumBytes,
            { RELEASE_V1_13.. => "max_transaction_size_in_bytes.gov" },
            1024 * 1024
        ],
        [
            gas_unit_scaling_factor: GasScalingFactor,
            "gas_unit_scaling_factor",
            GAS_SCALING_FACTOR
        ],
        // Gas Parameters for reading data from storage.
        [
            storage_io_per_state_slot_read: InternalGasPerArg,
            { 0..=9 => "load_data.base", 10.. => "storage_io_per_state_slot_read"},
            // At the current mainnet scale, we should assume most levels of the (hexary) JMT nodes
            // in cache, hence target charging 1-2 4k-sized pages for each read. Notice the cost
            // of seeking for the leaf node is covered by the first page of the "value size fee"
            // (storage_io_per_state_byte_read) defined below.
            302_385,
        ],
        [
            storage_io_per_state_byte_read: InternalGasPerByte,
            { 0..=9 => "load_data.per_byte", 10.. => "storage_io_per_state_byte_read"},
            // Notice in the latest IoPricing, bytes are charged at 4k intervals (even the smallest
            // read will be charged for 4KB) to reflect the assumption that every roughly 4k bytes
            // might require a separate random IO upon the FS.
            151,
        ],
        [load_data_failure: InternalGas, "load_data.failure", 0],
        // Gas parameters for writing data to storage.
        [
            storage_io_per_state_slot_write: InternalGasPerArg,
            { 0..=9 => "write_data.per_op", 10.. => "storage_io_per_state_slot_write"},
            // The cost of writing down the upper level new JMT nodes are shared between transactions
            // because we write down the JMT in batches, however the bottom levels will be specific
            // to each transactions assuming they don't touch exactly the same leaves. It's fair to
            // target roughly 1-2 full internal JMT nodes (about 0.5-1KB in total) worth of writes
            // for each write op.
            89_568,
        ],
        [
            legacy_write_data_per_new_item: InternalGasPerArg,
            {0..=9 => "write_data.new_item"},
            1_280_000,
        ],
        [
            storage_io_per_state_byte_write: InternalGasPerByte,
            { 0..=9 => "write_data.per_byte_in_key", 10.. => "storage_io_per_state_byte_write"},
            89,
        ],
        [
            legacy_write_data_per_byte_in_val: InternalGasPerByte,
            { 0..=9 => "write_data.per_byte_in_val" },
            10_000
        ],
        [
            storage_io_per_event_byte_write: InternalGasPerByte,
            { RELEASE_V1_11.. => "storage_io_per_event_byte_write" },
            89,
        ],
        [
            storage_io_per_transaction_byte_write: InternalGasPerByte,
            { RELEASE_V1_11.. => "storage_io_per_transaction_byte_write" },
            89,
        ],
        [memory_quota: AbstractValueSize, { 1.. => "memory_quota" }, 10_000_000],
        [
            legacy_free_write_bytes_quota: NumBytes,
            { 5.. => "free_write_bytes_quota" },
            1024, // 1KB free per state write
        ],
        [
            legacy_free_event_bytes_quota: NumBytes,
            { 7..=13 => "free_event_bytes_quota", 14.. => "legacy_free_event_bytes_quota" },
            1024, // 1KB free event bytes per transaction
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
            max_write_ops_per_transaction: NumSlots,
            { 11.. => "max_write_ops_per_transaction" },
            8192,
        ],
        [
            legacy_storage_fee_per_state_slot_create: FeePerSlot,
            { 7..=13 => "storage_fee_per_state_slot_create", 14.. => "legacy_storage_fee_per_state_slot_create" },
            50000,
        ],
        [
            storage_fee_per_state_slot: FeePerSlot,
            { 14.. => "storage_fee_per_state_slot" },
            // 0.8 million APT for 2 billion state slots
            40_000,
        ],
        [
            legacy_storage_fee_per_excess_state_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_excess_state_byte", 14.. => "legacy_storage_fee_per_excess_state_byte" },
            50,
        ],
        [
            storage_fee_per_state_byte: FeePerByte,
            { 14.. => "storage_fee_per_state_byte" },
            // 0.8 million APT for 2 TB state bytes
            40,
        ],
        [
            legacy_storage_fee_per_event_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_event_byte", 14.. => "legacy_storage_fee_per_event_byte" },
            20,
        ],
        [
            legacy_storage_fee_per_transaction_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_transaction_byte", 14.. => "legacy_storage_fee_per_transaction_byte" },
            20,
        ],
        [
            max_execution_gas: InternalGas,
            { 7.. => "max_execution_gas" },
            920_000_000, // 92ms of execution at 10k gas per ms
        ],
        [
            max_execution_gas_gov: InternalGas,
            { RELEASE_V1_13.. => "max_execution_gas.gov" },
            4_000_000_000,
        ],
        [
            max_io_gas: InternalGas,
            { 7.. => "max_io_gas" },
            1_000_000_000, // 100ms of IO at 10k gas per ms
        ],
        [
            max_io_gas_gov: InternalGas,
            { RELEASE_V1_13.. => "max_io_gas.gov" },
            2_000_000_000,
        ],
        [
            max_storage_fee: Fee,
            { 7.. => "max_storage_fee" },
            2_0000_0000, // 2 APT
        ],
        [
            max_storage_fee_gov: Fee,
            { RELEASE_V1_13.. => "max_storage_fee.gov" },
            2_0000_0000,
        ],
        [
            dependency_per_module: InternalGas,
            { RELEASE_V1_10.. => "dependency_per_module" },
            74460,
        ],
        [
            dependency_per_byte: InternalGasPerByte,
            { RELEASE_V1_10.. => "dependency_per_byte" },
            42,
        ],
        [
            max_num_dependencies: NumModules,
            { RELEASE_V1_10.. => "max_num_dependencies" },
            768,
        ],
        [
            max_total_dependency_size: NumBytes,
            { RELEASE_V1_10.. => "max_total_dependency_size" },
            1024 * 1024 * 18 / 10, // 1.8 MB
        ],
        [
            keyless_base_cost: InternalGas,
            { RELEASE_V1_12.. => "keyless.base" },
            32_000_000,
        ],
        [
            max_ty_size: NumTypeNodes,
            { RELEASE_V1_15.. => "max_ty_size" },
            128,
        ],
        [
            max_ty_depth: NumTypeNodes,
            { RELEASE_V1_15.. => "max_ty_depth" },
            20,
        ],
        [
            max_aa_gas: Gas,
            { RELEASE_V1_26.. => "max_aa_gas" },
            60,
        ]
    ]
);

use gas_params::*;

impl TransactionGasParameters {
    // TODO(Gas): Right now we are relying on this to avoid div by zero errors when using the all-zero
    //            gas parameters. See if there's a better way we can handle this.
    pub fn scaling_factor(&self) -> GasScalingFactor {
        match u64::from(self.gas_unit_scaling_factor) {
            0 => 1.into(),
            x => x.into(),
        }
    }

    /// Calculate the intrinsic gas for the transaction based upon its size in bytes.
    pub fn calculate_intrinsic_gas(
        &self,
        transaction_size: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + use<> {
        let excess = transaction_size
            .checked_sub(self.large_transaction_cutoff)
            .unwrap_or_else(|| 0.into());

        MIN_TRANSACTION_GAS_UNITS + INTRINSIC_GAS_PER_BYTE * excess
    }
}

impl ToUnitWithParams<TransactionGasParameters, InternalGasUnit> for GasUnit {
    fn multiplier(params: &TransactionGasParameters) -> u64 {
        params.scaling_factor().into()
    }
}
