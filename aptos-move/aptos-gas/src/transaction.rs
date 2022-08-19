// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::algebra::{FeePerGasUnit, Gas, GasScalingFactor, GasUnit};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerByte, InternalGasUnit, NumBytes, ToUnitFractionalWithParams,
    ToUnitWithParams,
};

crate::params::define_gas_parameters!(
    TransactionGasParameters,
    "txn",
    [
        [load_data_base: InternalGas, "load_data.base", 1],
        [
            load_data_per_byte: InternalGasPerByte,
            "load_data.per_byte",
            1
        ],
        [load_data_failure: InternalGas, "load_data.failure", 1],
        // The flat minimum amount of gas required for any transaction.
        // Charged at the start of execution.
        [
            min_transaction_gas_units: InternalGas,
            "min_transaction_gas_units",
            600
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
            8
        ],
        // ~5 microseconds should equal one unit of computational gas. We bound the maximum
        // computational time of any given transaction at roughly 20 seconds. We want this number and
        // `MAX_PRICE_PER_GAS_UNIT` to always satisfy the inequality that
        // MAXIMUM_NUMBER_OF_GAS_UNITS * MAX_PRICE_PER_GAS_UNIT < min(u64::MAX, GasUnits<GasCarrier>::MAX)
        [
            maximum_number_of_gas_units: Gas,
            "maximum_number_of_gas_units",
            4_000_000
        ],
        // The minimum gas price that a transaction can be submitted with.
        // TODO(Gas): should probably change this to something > 0
        [
            min_price_per_gas_unit: FeePerGasUnit,
            "min_price_per_gas_unit",
            0
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
            6 * 1024 * 1024
        ],
        [
            gas_unit_scaling_factor: GasScalingFactor,
            "gas_unit_scaling_factor",
            1000
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
