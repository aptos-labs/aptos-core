// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::gas_schedule::VMGasParameters;
use aptos_gas_algebra::{
    AbstractValueSize, Fee, FeePerByte, FeePerGasUnit, FeePerSlot, Gas, GasExpression,
    GasScalingFactor, GasUnit, NumSlots,
};
use aptos_types::{
    contract_event::ContractEvent, state_store::state_key::StateKey, write_set::WriteOpSize,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, InternalGasUnit, NumBytes, ToUnitWithParams,
};

const GAS_SCALING_FACTOR: u64 = 1_000_000;

const GAS_INTRINSIC_MULTIPLIER: u64 = 1_000_000;
const GAS_EXECUTION_MULTIPLIER: u64 = 1_000_000;
const GAS_IO_READ_MULTIPLIER: u64 = 1_000_000;
const GAS_IO_WRITE_MULTIPLIER: u64 = 1_000_000;

const fn adjust(value: u64, factor: u64) -> u64 {
    (value as u128 * factor as u128 / 1_000_000) as u64
}

crate::gas_schedule::macros::define_gas_parameters!(
    TransactionGasParameters,
    "txn",
    VMGasParameters => .txn,
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
            adjust(2_000, GAS_INTRINSIC_MULTIPLIER),
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
            gas_unit_scaling_factor: GasScalingFactor,
            "gas_unit_scaling_factor",
            GAS_SCALING_FACTOR
        ],
        // Gas Parameters for reading data from storage.
        [
            storage_io_per_state_slot_read: InternalGasPerArg,
            { 0..=9 => "load_data.base", 10.. => "storage_io_per_state_slot_read"},
            adjust(300_000, GAS_IO_READ_MULTIPLIER),
        ],
        [
            storage_io_per_state_byte_read: InternalGasPerByte,
            { 0..=9 => "load_data.per_byte", 10.. => "storage_io_per_state_byte_read"},
            adjust(300, GAS_IO_READ_MULTIPLIER),
        ],
        [load_data_failure: InternalGas, "load_data.failure", 0],
        // Gas parameters for writing data to storage.
        [
            storage_io_per_state_slot_write: InternalGasPerArg,
            { 0..=9 => "write_data.per_op", 10.. => "storage_io_per_state_slot_write"},
            adjust(300_000, GAS_IO_WRITE_MULTIPLIER),
        ],
        [
            write_data_per_new_item: InternalGasPerArg,
            "write_data.new_item",
            adjust(1_280_000, GAS_IO_WRITE_MULTIPLIER),
        ],
        [
            storage_io_per_state_byte_write: InternalGasPerByte,
            { 0..=9 => "write_data.per_byte_in_key", 10.. => "storage_io_per_state_byte_write"},
            adjust(5_000, GAS_IO_WRITE_MULTIPLIER),
        ],
        [
            write_data_per_byte_in_val: InternalGasPerByte,
            "write_data.per_byte_in_val",
            adjust(10_000, GAS_IO_WRITE_MULTIPLIER),
        ],
        [memory_quota: AbstractValueSize, { 1.. => "memory_quota" }, 10_000_000],
        [
            free_write_bytes_quota: NumBytes,
            { 5.. => "free_write_bytes_quota" },
            1024, // 1KB free per state write
        ],
        [
            free_event_bytes_quota: NumBytes,
            { 7.. => "free_event_bytes_quota" },
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
            storage_fee_per_state_slot_create: FeePerSlot,
            { 7.. => "storage_fee_per_state_slot_create" },
            50000,
        ],
        [
            storage_fee_per_excess_state_byte: FeePerByte,
            { 7.. => "storage_fee_per_excess_state_byte" },
            50,
        ],
        [
            storage_fee_per_event_byte: FeePerByte,
            { 7.. => "storage_fee_per_event_byte" },
            20,
        ],
        [
            storage_fee_per_transaction_byte: FeePerByte,
            { 7.. => "storage_fee_per_transaction_byte" },
            20,
        ],
        [
            max_execution_gas: InternalGas,
            { 7.. => "max_execution_gas" },
            20_000_000_000,
        ],
        [
            max_io_gas: InternalGas,
            { 7.. => "max_io_gas" },
            10_000_000_000,
        ],
        [
            max_storage_fee: Fee,
            { 7.. => "max_storage_fee" },
            2_0000_0000, // 2 APT
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

    pub fn storage_fee_for_slot(&self, op_size: &WriteOpSize) -> Fee {
        use WriteOpSize::*;

        match op_size {
            Creation { .. } => self.storage_fee_per_state_slot_create * NumSlots::new(1),
            Modification { .. } | Deletion | DeletionWithDeposit { .. } => 0.into(),
        }
    }

    pub fn storage_fee_refund_for_slot(&self, op_size: &WriteOpSize) -> Fee {
        op_size.deletion_deposit().map_or(0.into(), Fee::new)
    }

    /// Maybe value size is None for deletion Ops.
    pub fn storage_fee_for_bytes(&self, key: &StateKey, op_size: &WriteOpSize) -> Fee {
        if let Some(value_size) = op_size.write_len() {
            let size = NumBytes::new(key.size() as u64) + NumBytes::new(value_size);
            if let Some(excess) = size.checked_sub(self.free_write_bytes_quota) {
                return excess * self.storage_fee_per_excess_state_byte;
            }
        }

        0.into()
    }

    /// New formula to charge storage fee for an event, measured in APT.
    pub fn storage_fee_per_event(&self, event: &ContractEvent) -> Fee {
        NumBytes::new(event.size() as u64) * self.storage_fee_per_event_byte
    }

    pub fn storage_discount_for_events(&self, total_cost: Fee) -> Fee {
        std::cmp::min(
            total_cost,
            self.free_event_bytes_quota * self.storage_fee_per_event_byte,
        )
    }

    /// New formula to charge storage fee for transaction, measured in APT.
    pub fn storage_fee_for_transaction_storage(&self, txn_size: NumBytes) -> Fee {
        txn_size
            .checked_sub(self.large_transaction_cutoff)
            .unwrap_or(NumBytes::zero())
            * self.storage_fee_per_transaction_byte
    }

    /// Calculate the intrinsic gas for the transaction based upon its size in bytes.
    pub fn calculate_intrinsic_gas(
        &self,
        transaction_size: NumBytes,
    ) -> impl GasExpression<VMGasParameters, Unit = InternalGasUnit> {
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
