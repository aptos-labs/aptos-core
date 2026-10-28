// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines all the gas parameters for transactions, along with their initial values
//! in the genesis and a mapping between the Rust representation and the on-chain gas schedule.

use crate::{
    gas_schedule::VMGasParameters,
    ver::gas_feature_versions::{
        RELEASE_V1_10, RELEASE_V1_11, RELEASE_V1_12, RELEASE_V1_13, RELEASE_V1_15, RELEASE_V1_26,
        RELEASE_V1_41, RELEASE_V1_45, RELEASE_V1_50,
    },
};
use aptos_gas_algebra::{
    AbstractValueSize, BytecodeVersion, Fee, FeePerByte, FeePerGasUnit, FeePerSlot, Gas,
    GasExpression, GasMultiplier, GasScalingFactor, GasUnit, NumModules, NumSlots, NumTypeNodes,
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
            27_600_000
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
            11_580
        ],
        // We want this number and `MAX_PRICE_PER_GAS_UNIT` to always satisfy the inequality that
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
        [
            high_limit_txn_min_price_per_gas_unit: FeePerGasUnit,
            { RELEASE_V1_45.. => "high_limit_txn_min_price_per_gas_unit" },
            aptos_global_constants::GAS_UNIT_PRICE
        ],
        // Transactions with a staking-backed limits request pay this many times
        // the gas price for gas used above the base execution and IO limits.
        [
            high_limit_txn_excess_price_multiplier: GasMultiplier,
            { RELEASE_V1_50.. => "high_limit_txn_excess_price_multiplier" },
            10
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
            3_023_850,
        ],
        [
            storage_io_per_state_byte_read: InternalGasPerByte,
            { 0..=9 => "load_data.per_byte", 10.. => "storage_io_per_state_byte_read"},
            // Notice in the latest IoPricing, bytes are charged at 4k intervals (even the smallest
            // read will be charged for 4KB) to reflect the assumption that every roughly 4k bytes
            // might require a separate random IO upon the FS.
            1510,
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
            895_680,
        ],
        [
            legacy_write_data_per_new_item: InternalGasPerArg,
            {0..=9 => "write_data.new_item"},
            12_800_000,
        ],
        [
            storage_io_per_state_byte_write: InternalGasPerByte,
            { 0..=9 => "write_data.per_byte_in_key", 10.. => "storage_io_per_state_byte_write"},
            890,
        ],
        [
            legacy_write_data_per_byte_in_val: InternalGasPerByte,
            { 0..=9 => "write_data.per_byte_in_val" },
            100_000
        ],
        [
            storage_io_per_event_byte_write: InternalGasPerByte,
            { RELEASE_V1_11.. => "storage_io_per_event_byte_write" },
            890,
        ],
        [
            storage_io_per_transaction_byte_write: InternalGasPerByte,
            { RELEASE_V1_11.. => "storage_io_per_transaction_byte_write" },
            890,
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
            500000,
        ],
        [
            storage_fee_per_state_slot: FeePerSlot,
            { 14.. => "storage_fee_per_state_slot" },
            // 8 million APT for 2 billion state slots
            400_000,
        ],
        [
            legacy_storage_fee_per_excess_state_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_excess_state_byte", 14.. => "legacy_storage_fee_per_excess_state_byte" },
            500,
        ],
        [
            storage_fee_per_state_byte: FeePerByte,
            { 14.. => "storage_fee_per_state_byte" },
            // 8 million APT for 2 TB state bytes
            400,
        ],
        [
            legacy_storage_fee_per_event_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_event_byte", 14.. => "legacy_storage_fee_per_event_byte" },
            200,
        ],
        [
            legacy_storage_fee_per_transaction_byte: FeePerByte,
            { 7..=13 => "storage_fee_per_transaction_byte", 14.. => "legacy_storage_fee_per_transaction_byte" },
            200,
        ],
        [
            max_execution_gas: InternalGas,
            { 7.. => "max_execution_gas" },
            9_200_000_000, // 92ms of execution at 100k gas per ms
        ],
        [
            max_execution_gas_gov: InternalGas,
            { RELEASE_V1_13.. => "max_execution_gas.gov" },
            40_000_000_000,
        ],
        [
            max_io_gas: InternalGas,
            { 7.. => "max_io_gas" },
            10_000_000_000, // 100ms of IO at 100k gas per ms
        ],
        [
            max_io_gas_gov: InternalGas,
            { RELEASE_V1_13.. => "max_io_gas.gov" },
            20_000_000_000,
        ],
        [
            max_storage_fee: Fee,
            { 7.. => "max_storage_fee" },
            20_0000_0000, // 20 APT
        ],
        [
            max_storage_fee_gov: Fee,
            { RELEASE_V1_13.. => "max_storage_fee.gov" },
            20_0000_0000,
        ],
        [
            dependency_per_module: InternalGas,
            { RELEASE_V1_10.. => "dependency_per_module" },
            744600,
        ],
        [
            dependency_per_byte: InternalGasPerByte,
            { RELEASE_V1_10.. => "dependency_per_byte" },
            420,
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
            320_000_000,
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
            600,
        ],
        [
            slh_dsa_sha2_128s_base_cost: InternalGas,
            { RELEASE_V1_41.. => "slh_dsa_sha2_128s.base" },
            138_000_000,
        ],
        [
            encrypted_txn_decryption_base_cost: InternalGas,
            { RELEASE_V1_45.. => "encrypted_txn_decryption.base" },
            // 120ms e2e decryption latency amortized over 32 txns * 100M internal gas/ms.
            // The per-block max is 64 encrypted txns, so we assume a half-full block.
            375_000_000,
        ],
        [
            encrypted_txn_min_price_per_gas_unit: FeePerGasUnit,
            { RELEASE_V1_45.. => "encrypted_txn_min_price_per_gas_unit" },
            200,  // 2x the current min_price_per_gas_unit (100)
        ],
        // Modules with a bytecode (file format) version below this cannot be published.
        // Only publishing is gated: modules already on chain keep loading and executing at any
        // version. Policy: permit the latest version and the two before it (VERSION_MAX - 2).
        [
            min_module_bytecode_version: BytecodeVersion,
            { RELEASE_V1_50.. => "min_module_bytecode_version" },
            8, // VERSION_MAX (10) - 2
        ],
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

/// Pricing for transactions with a staking-backed limits request. Gas used
/// above the base execution and IO limits costs a higher gas price.
#[derive(Clone, Copy, Debug)]
pub struct HighLimitPricing {
    /// Multiplier for gas price to charge for any gas used above base
    /// execution and IO limits.
    excess_gas_price_multiplier: u64,
    base_max_execution_gas: Gas,
    base_max_io_gas: Gas,
}

impl HighLimitPricing {
    /// Returns [`None`] if high limit pricing is not enabled.
    pub fn new(gas_feature_version: u64, params: &TransactionGasParameters) -> Option<Self> {
        if gas_feature_version < RELEASE_V1_50 {
            return None;
        }
        Some(Self {
            // Price multiplier must be at least 1x.
            excess_gas_price_multiplier: u64::from(params.high_limit_txn_excess_price_multiplier)
                .max(1),
            base_max_execution_gas: params
                .max_execution_gas
                .to_unit_round_up_with_params(params),
            base_max_io_gas: params.max_io_gas.to_unit_round_up_with_params(params),
        })
    }

    /// The multiple of the gas price that gas used above the base execution
    /// and IO limits costs.
    pub fn excess_gas_price_multiplier(&self) -> u64 {
        self.excess_gas_price_multiplier
    }

    /// The most the transaction can be charged, or `None` if it overflows.
    pub fn max_fee(&self, gas_unit_price: u64, max_gas_units: u64) -> Option<u64> {
        gas_unit_price
            .checked_mul(self.excess_gas_price_multiplier)?
            .checked_mul(max_gas_units)
    }

    /// The gas price the prologue reserves the maximum fee at.
    pub fn reserve_price(&self, gas_unit_price: u64) -> u64 {
        gas_unit_price.saturating_mul(self.excess_gas_price_multiplier)
    }

    /// The maximum gas units the epilogue bills against. Each excess unit is
    /// counted `k` times, so billing at the gas price charges it at `k` times
    /// the price. Bounded by `k * max_gas_units`.
    pub fn epilogue_max_gas_units(
        &self,
        max_gas_units: u64,
        gas_used: u64,
        execution_gas_used: u64,
        io_gas_used: u64,
    ) -> u64 {
        // Execution and IO are rounded up separately, so their sum can exceed
        // the gas used.
        let excess = execution_gas_used
            .saturating_sub(self.base_max_execution_gas.into())
            .saturating_add(io_gas_used.saturating_sub(self.base_max_io_gas.into()))
            .min(gas_used);
        max_gas_units.saturating_add((self.excess_gas_price_multiplier - 1).saturating_mul(excess))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ver::gas_feature_versions::RELEASE_V1_49, InitialGasSchedule};

    // Base limits of 1,000 execution and 2,000 IO gas units.
    fn high_limit_pricing(multiplier: u64) -> HighLimitPricing {
        let mut params = TransactionGasParameters::initial();
        params.max_execution_gas = Gas::new(1_000).to_unit_with_params(&params);
        params.max_io_gas = Gas::new(2_000).to_unit_with_params(&params);
        params.high_limit_txn_excess_price_multiplier = multiplier.into();
        HighLimitPricing::new(RELEASE_V1_50, &params).unwrap()
    }

    #[test]
    fn high_limit_pricing_disabled_before_v1_50() {
        let params = TransactionGasParameters::initial();
        assert!(HighLimitPricing::new(RELEASE_V1_49, &params).is_none());
    }

    #[test]
    fn high_limit_pricing_rounds_base_limits_up() {
        let mut params = TransactionGasParameters::initial();
        params.max_execution_gas = InternalGas::new(1_000 * GAS_SCALING_FACTOR + 1);
        params.max_io_gas = InternalGas::new(2_000 * GAS_SCALING_FACTOR - 1);
        let pricing = HighLimitPricing::new(RELEASE_V1_50, &params).unwrap();
        assert_eq!(pricing.base_max_execution_gas, Gas::new(1_001));
        assert_eq!(pricing.base_max_io_gas, Gas::new(2_000));
    }

    #[test]
    fn high_limit_pricing_within_base_limits() {
        let pricing = high_limit_pricing(10);
        assert_eq!(
            pricing.epilogue_max_gas_units(10_000, 3_000, 1_000, 2_000),
            10_000
        );
    }

    #[test]
    fn high_limit_pricing_execution_excess() {
        let pricing = high_limit_pricing(10);
        // 500 execution units above the base, each counted 10 times.
        assert_eq!(
            pricing.epilogue_max_gas_units(10_000, 3_500, 1_500, 2_000),
            10_000 + 9 * 500
        );
    }

    #[test]
    fn high_limit_pricing_io_excess() {
        let pricing = high_limit_pricing(10);
        assert_eq!(
            pricing.epilogue_max_gas_units(10_000, 3_300, 1_000, 2_300),
            10_000 + 9 * 300
        );
    }

    #[test]
    fn high_limit_pricing_excess_capped_by_gas_used() {
        let pricing = high_limit_pricing(10);
        // Rounding up execution and IO separately overshoots the gas used.
        assert_eq!(
            pricing.epilogue_max_gas_units(10_000, 400, 1_300, 2_300),
            10_000 + 9 * 400
        );
    }

    #[test]
    fn high_limit_pricing_fee_never_exceeds_reserve() {
        let pricing = high_limit_pricing(10);
        let (price, max_gas_units) = (100, 10_000);
        for (gas_used, exec, io) in [(0, 0, 0), (3_000, 1_000, 2_000), (10_000, 9_000, 9_000)] {
            let remaining = max_gas_units - gas_used;
            let billed = pricing.epilogue_max_gas_units(max_gas_units, gas_used, exec, io);
            let fee = price * (billed - remaining);
            assert!(fee <= pricing.reserve_price(price) * max_gas_units);
        }
    }

    #[test]
    fn high_limit_pricing_multiplier_at_most_1x() {
        for multiplier in [0, 1] {
            let pricing = high_limit_pricing(multiplier);
            assert_eq!(pricing.reserve_price(100), 100);
            assert_eq!(
                pricing.epilogue_max_gas_units(10_000, 3_500, 1_500, 2_000),
                10_000
            );
        }
    }

    #[test]
    fn high_limit_pricing_max_fee_overflow() {
        let pricing = high_limit_pricing(10);
        assert_eq!(pricing.max_fee(100, 10_000), Some(10_000_000));
        assert_eq!(pricing.max_fee(u64::MAX / 10, 2), None);
    }

    /// Encrypted transactions must pay at least 2x the base minimum gas unit price
    /// to reflect their block priority. If governance raises `min_price_per_gas_unit`
    /// without bumping `encrypted_txn_min_price_per_gas_unit` proportionally, the
    /// encrypted premium collapses.
    ///
    /// In test builds `aptos_global_constants::GAS_UNIT_PRICE` is `0`, so comparing against
    /// `TransactionGasParameters::initial().min_price_per_gas_unit` directly would make this
    /// check trivially pass. Instead we simulate mainnet by overriding `min_price_per_gas_unit`
    /// with its prod value (100) and verify the invariant.
    #[test]
    fn encrypted_min_price_is_at_least_2x_base_min_price() {
        const MAINNET_MIN_PRICE_PER_GAS_UNIT: u64 = 100;

        let mut params = TransactionGasParameters::initial();
        params.min_price_per_gas_unit = MAINNET_MIN_PRICE_PER_GAS_UNIT.into();

        let base: u64 = u64::from(params.min_price_per_gas_unit);
        let encrypted: u64 = u64::from(params.encrypted_txn_min_price_per_gas_unit);
        assert!(
            encrypted >= base.saturating_mul(2),
            "encrypted_txn_min_price_per_gas_unit ({}) must be at least 2x min_price_per_gas_unit ({})",
            encrypted,
            base,
        );
    }
}
