// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::traits::GasAlgebra;
use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, NumBytes, NumModules, Octa};
use aptos_gas_schedule::{gas_feature_versions, VMGasParameters};
use aptos_logger::error;
use aptos_types::transaction::TxnLimitsRequest;
use aptos_vm_types::{
    resolver::BlockSynchronizationKillSwitch,
    storage::{io_pricing::IoPricing, space_pricing::DiskSpacePricing, StorageGasParameters},
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    gas_algebra::{InternalGas, InternalGasUnit},
    vm_status::StatusCode,
};
use std::{fmt::Debug, ops::AddAssign};

/// Limit on execution or IO gas in billed units, and the price multiplier for
/// gas used under it.
#[derive(Clone, Copy)]
struct GasTier {
    limit: InternalGas,
    multiplier: u64,
}

impl GasTier {
    /// Returns the tier that charges 1x up to `base`, and the higher-priced
    /// tiers after it: `multiplier` times from `base` up to `max`.
    fn with_excess(base: InternalGas, max: InternalGas, multiplier: u64) -> (Self, Vec<Self>) {
        if multiplier <= 1 || max <= base {
            let tier = Self {
                limit: max,
                multiplier: 1,
            };
            return (tier, vec![]);
        }
        let excess = u64::from(max) - u64::from(base);
        let limit = base + InternalGas::new(excess.saturating_mul(multiplier));
        (
            Self {
                limit: base,
                multiplier: 1,
            },
            vec![Self { limit, multiplier }],
        )
    }

    /// Returns the billed amount for `amount` of gas used in this tier.
    #[inline(always)]
    fn price(&self, amount: InternalGas) -> InternalGas {
        InternalGas::new(u64::from(amount).saturating_mul(self.multiplier))
    }
}

/// Moves to the next tier while `used` is above the current tier's limit.
/// Gas used above the limit was charged at the current tier's price, so the
/// difference to the next tier's price is charged on top.
fn exceed_tier_limit(
    balance: &mut InternalGas,
    used: &mut InternalGas,
    tier: &mut GasTier,
    next_tiers: &mut Vec<GasTier>,
    limit_reached: StatusCode,
) -> PartialVMResult<()> {
    while *used > tier.limit {
        let next = next_tiers
            .pop()
            .ok_or_else(|| PartialVMError::new(limit_reached))?;

        // Exact: usage enters a tier at its lower bound and grows in multiples
        // of the tier's multiplier.
        let excess = (u64::from(*used) - u64::from(tier.limit)) / tier.multiplier;
        let extra = InternalGas::new(
            excess.saturating_mul(next.multiplier.saturating_sub(tier.multiplier)),
        );
        match balance.checked_sub(extra) {
            Some(new_balance) => {
                *balance = new_balance;
                *used += extra;
            },
            None => {
                *used += *balance;
                *balance = 0.into();
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            },
        }
        *tier = next;
    }
    Ok(())
}

/// Base gas algebra implementation that tracks the gas usage using its internal counters.
///
/// Abstract gas amounts are always evaluated to concrete values at the spot.
pub struct StandardGasAlgebra<'a, T>
where
    T: BlockSynchronizationKillSwitch,
{
    feature_version: u64,
    vm_gas_params: VMGasParameters,
    storage_gas_params: StorageGasParameters,

    initial_balance: InternalGas,
    balance: InternalGas,

    execution_tier: GasTier,
    // Higher-priced execution tiers, in reverse order.
    next_execution_tiers: Vec<GasTier>,
    execution_gas_used: InternalGas,

    io_tier: GasTier,
    // Higher-priced IO tiers, in reverse order.
    next_io_tiers: Vec<GasTier>,
    io_gas_used: InternalGas,

    max_storage_fee: Fee,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,

    num_dependencies: NumModules,
    total_dependency_size: NumBytes,

    // Block synchronization kill switch allows checking whether the ongoing execution should
    // be interrupted, due to external (block execution related) conditions (such as block gas
    // limit being reached). Interrupting is a performance optimization, and requires checking
    // with proper granularity. Gas charging happens regularly but involves computation that
    // can amortize the cost of the check. Hence, currently kill switch is integrated here.
    block_synchronization_kill_switch: &'a T,
    // To control the performance overhead, kill switch is checked one out of (4) times in
    // gas charging callback.
    counter_for_kill_switch: usize,
}

impl<'a, T> StandardGasAlgebra<'a, T>
where
    T: BlockSynchronizationKillSwitch,
{
    pub fn new(
        gas_feature_version: u64,
        vm_gas_params: VMGasParameters,
        storage_gas_params: StorageGasParameters,
        txn_limits_request: Option<&TxnLimitsRequest>,
        balance: impl Into<Gas>,
        block_synchronization_kill_switch: &'a T,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&vm_gas_params.txn);

        let (max_execution_gas, max_io_gas, max_storage_fee) = match txn_limits_request {
            Some(TxnLimitsRequest::ApprovedGovernanceScript)
                if gas_feature_version >= gas_feature_versions::RELEASE_V1_13 =>
            {
                (
                    vm_gas_params.txn.max_execution_gas_gov,
                    vm_gas_params.txn.max_io_gas_gov,
                    vm_gas_params.txn.max_storage_fee_gov,
                )
            },
            Some(TxnLimitsRequest::Staking(request)) => {
                // Multipliers are expressed as percent of the base limit
                // (100 = 1x).
                let m = request.multipliers();
                let max_execution_gas = u64::from(vm_gas_params.txn.max_execution_gas)
                    .saturating_mul(m.execution_multiplier_percent())
                    / 100;
                let max_io_gas = u64::from(vm_gas_params.txn.max_io_gas)
                    .saturating_mul(m.io_multiplier_percent())
                    / 100;

                (
                    InternalGas::new(max_execution_gas),
                    InternalGas::new(max_io_gas),
                    // Storage limits are kept as is.
                    vm_gas_params.txn.max_storage_fee,
                )
            },
            // Pre v1.13 governance scripts or no request: use standard limits.
            None | Some(TxnLimitsRequest::ApprovedGovernanceScript) => (
                vm_gas_params.txn.max_execution_gas,
                vm_gas_params.txn.max_io_gas,
                vm_gas_params.txn.max_storage_fee,
            ),
        };

        // Staking-backed requests pay a higher price for gas above the base
        // limits.
        let excess_multiplier = match txn_limits_request {
            Some(TxnLimitsRequest::Staking(_))
                if gas_feature_version >= gas_feature_versions::RELEASE_V1_51 =>
            {
                u64::from(vm_gas_params.txn.high_limit_txn_excess_price_multiplier)
            },
            None
            | Some(TxnLimitsRequest::Staking(_))
            | Some(TxnLimitsRequest::ApprovedGovernanceScript) => 1,
        };
        let (execution_tier, next_execution_tiers) = GasTier::with_excess(
            vm_gas_params.txn.max_execution_gas,
            max_execution_gas,
            excess_multiplier,
        );
        let (io_tier, next_io_tiers) =
            GasTier::with_excess(vm_gas_params.txn.max_io_gas, max_io_gas, excess_multiplier);

        Self {
            feature_version: gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            initial_balance: balance,
            balance,
            execution_tier,
            next_execution_tiers,
            execution_gas_used: 0.into(),
            io_tier,
            next_io_tiers,
            io_gas_used: 0.into(),
            max_storage_fee,
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
            num_dependencies: 0.into(),
            total_dependency_size: 0.into(),
            block_synchronization_kill_switch,
            counter_for_kill_switch: 0,
        }
    }
}

impl<T> GasAlgebra for StandardGasAlgebra<'_, T>
where
    T: BlockSynchronizationKillSwitch,
{
    fn feature_version(&self) -> u64 {
        self.feature_version
    }

    fn vm_gas_params(&self) -> &VMGasParameters {
        &self.vm_gas_params
    }

    fn storage_gas_params(&self) -> &StorageGasParameters {
        &self.storage_gas_params
    }

    fn io_pricing(&self) -> &IoPricing {
        &self.storage_gas_params.io_pricing
    }

    fn disk_space_pricing(&self) -> &DiskSpacePricing {
        &self.storage_gas_params.space_pricing
    }

    fn balance_internal(&self) -> InternalGas {
        self.balance
    }

    fn check_consistency(&self) -> PartialVMResult<()> {
        let total = self
            .initial_balance
            .checked_sub(self.balance)
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "Current balance ({}) exceedes the initial balance ({}) -- how is this ever possible?",
                        self.balance,
                        self.initial_balance
                    ),
                )
            })?;

        let total_calculated =
            self.execution_gas_used + self.io_gas_used + self.storage_fee_in_internal_units;
        if total != total_calculated {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "The per-category costs do not add up. {} (total) != {} = {} (exec) + {} (io) + {} (storage)",
                        total,
                        total_calculated,
                        self.execution_gas_used,
                        self.io_gas_used,
                        self.storage_fee_in_internal_units,
                    ),
                ),
            );
        }

        Ok(())
    }

    #[inline(always)]
    fn charge_execution(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit> + Debug,
    ) -> PartialVMResult<()> {
        self.counter_for_kill_switch += 1;
        if self.counter_for_kill_switch & 3 == 0
            && self.block_synchronization_kill_switch.interrupt_requested()
        {
            return Err(
                PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR)
                    .with_message("Interrupted from block synchronization view".to_string()),
            );
        }

        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);
        let amount = self.execution_tier.price(amount);

        match self.balance.checked_sub(amount) {
            Some(new_balance) => {
                self.balance = new_balance;
                self.execution_gas_used += amount;
            },
            None => {
                let old_balance = self.balance;
                self.balance = 0.into();
                if self.feature_version >= 12 {
                    self.execution_gas_used += old_balance;
                }
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            },
        };

        if self.feature_version >= 7 && self.execution_gas_used > self.execution_tier.limit {
            exceed_tier_limit(
                &mut self.balance,
                &mut self.execution_gas_used,
                &mut self.execution_tier,
                &mut self.next_execution_tiers,
                StatusCode::EXECUTION_LIMIT_REACHED,
            )
        } else {
            Ok(())
        }
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);
        let amount = self.io_tier.price(amount);

        match self.balance.checked_sub(amount) {
            Some(new_balance) => {
                self.balance = new_balance;
                self.io_gas_used += amount;
            },
            None => {
                let old_balance = self.balance;
                self.balance = 0.into();
                if self.feature_version >= 12 {
                    self.io_gas_used += old_balance;
                }
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            },
        };

        if self.feature_version >= 7 && self.io_gas_used > self.io_tier.limit {
            exceed_tier_limit(
                &mut self.balance,
                &mut self.io_gas_used,
                &mut self.io_tier,
                &mut self.next_io_tiers,
                StatusCode::IO_LIMIT_REACHED,
            )
        } else {
            Ok(())
        }
    }

    fn charge_storage_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        let txn_params = &self.vm_gas_params.txn;

        // Because the storage fees are defined in terms of fixed APT costs, we need
        // to convert them into gas units.
        //
        // u128 is used to protect against overflow and preserve as much precision as
        // possible in the extreme cases.
        fn div_ceil(n: u128, d: u128) -> u128 {
            if n.is_multiple_of(d) {
                n / d
            } else {
                n / d + 1
            }
        }
        let gas_consumed_internal = div_ceil(
            (u64::from(amount) as u128) * (u64::from(txn_params.gas_unit_scaling_factor) as u128),
            u64::from(gas_unit_price) as u128,
        );
        let gas_consumed_internal = InternalGas::new(
            if gas_consumed_internal > u64::MAX as u128 {
                error!(
                    "Something's wrong in the gas schedule: gas_consumed_internal ({}) > u64::MAX",
                    gas_consumed_internal
                );
                u64::MAX
            } else {
                gas_consumed_internal as u64
            },
        );

        match self.balance.checked_sub(gas_consumed_internal) {
            Some(new_balance) => {
                self.balance = new_balance;
                self.storage_fee_in_internal_units += gas_consumed_internal;
                self.storage_fee_used += amount;
            },
            None => {
                let old_balance = self.balance;
                self.balance = 0.into();
                if self.feature_version >= 12 {
                    self.storage_fee_in_internal_units += old_balance;
                    self.storage_fee_used += amount;
                }
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            },
        };

        if self.feature_version >= 7 && self.storage_fee_used > self.max_storage_fee {
            return Err(PartialVMError::new(StatusCode::STORAGE_LIMIT_REACHED));
        }

        Ok(())
    }

    fn count_dependency(&mut self, size: NumBytes) -> PartialVMResult<()> {
        if self.feature_version >= 15 {
            self.num_dependencies += 1.into();
            self.total_dependency_size += size;

            if self.num_dependencies > self.vm_gas_params.txn.max_num_dependencies {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
            if self.total_dependency_size > self.vm_gas_params.txn.max_total_dependency_size {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
        }
        Ok(())
    }

    fn execution_gas_used(&self) -> InternalGas {
        self.execution_gas_used
    }

    fn io_gas_used(&self) -> InternalGas {
        self.io_gas_used
    }

    fn storage_fee_used_in_gas_units(&self) -> InternalGas {
        self.storage_fee_in_internal_units
    }

    fn storage_fee_used(&self) -> Fee {
        self.storage_fee_used
    }

    // Reset the initial gas balance to reflect the new balance with the change carried over.
    fn inject_balance(&mut self, extra_balance: impl Into<Gas>) -> PartialVMResult<()> {
        let extra_unit = extra_balance
            .into()
            .to_unit_with_params(&self.vm_gas_params.txn);
        self.initial_balance.add_assign(extra_unit);
        self.balance.add_assign(extra_unit);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_gas_schedule::{InitialGasSchedule, LATEST_GAS_FEATURE_VERSION};
    use aptos_types::transaction::{RequestedMultipliers, UserTxnLimitsRequest};
    use aptos_vm_types::resolver::NoopBlockSynchronizationKillSwitch;

    const KILL_SWITCH: NoopBlockSynchronizationKillSwitch = NoopBlockSynchronizationKillSwitch {};

    fn staking_request(multiplier_percent: u64) -> TxnLimitsRequest {
        TxnLimitsRequest::Staking(UserTxnLimitsRequest::StakePoolOwner {
            multipliers: RequestedMultipliers::V1 {
                execution_multiplier_percent: multiplier_percent,
                io_multiplier_percent: multiplier_percent,
            },
        })
    }

    // Balance of 1,000,000 internal units. Base limits of 100,000 internal
    // units, and gas above the base costs 10x.
    fn new_algebra(
        gas_feature_version: u64,
        request: Option<&TxnLimitsRequest>,
    ) -> StandardGasAlgebra<'static, NoopBlockSynchronizationKillSwitch> {
        let mut params = VMGasParameters::initial();
        params.txn.max_execution_gas = 100_000.into();
        params.txn.max_io_gas = 100_000.into();
        params.txn.high_limit_txn_excess_price_multiplier = 10.into();
        StandardGasAlgebra::new(
            gas_feature_version,
            params,
            StorageGasParameters::latest(),
            request,
            Gas::new(1),
            &KILL_SWITCH,
        )
    }

    fn assert_used(
        algebra: &StandardGasAlgebra<NoopBlockSynchronizationKillSwitch>,
        execution: u64,
        io: u64,
    ) {
        assert_eq!(algebra.execution_gas_used(), execution.into());
        assert_eq!(algebra.io_gas_used(), io.into());
        assert_eq!(
            algebra.balance_internal(),
            (1_000_000 - execution - io).into()
        );
        assert!(algebra.check_consistency().is_ok());
    }

    #[test]
    fn excess_above_base_limit_costs_multiplier() {
        let request = staking_request(400);
        let mut algebra = new_algebra(LATEST_GAS_FEATURE_VERSION, Some(&request));

        assert!(algebra.charge_execution(InternalGas::new(60_000)).is_ok());
        assert_used(&algebra, 60_000, 0);

        // Crosses the base limit: 40,000 at 1x and 20,000 at 10x.
        assert!(algebra.charge_execution(InternalGas::new(60_000)).is_ok());
        assert_used(&algebra, 300_000, 0);

        assert!(algebra.charge_execution(InternalGas::new(1_000)).is_ok());
        assert_used(&algebra, 310_000, 0);

        // IO is priced separately.
        assert!(algebra.charge_io(InternalGas::new(100_000)).is_ok());
        assert_used(&algebra, 310_000, 100_000);
        assert!(algebra.charge_io(InternalGas::new(1_000)).is_ok());
        assert_used(&algebra, 310_000, 110_000);
    }

    #[test]
    fn excess_above_max_limit_reaches_limit() {
        let request = staking_request(110);
        let mut algebra = new_algebra(LATEST_GAS_FEATURE_VERSION, Some(&request));

        // Crosses both limits: 100,000 at 1x and 20,000 at 10x, above the max
        // of 110,000 (billed as 200,000).
        let err = algebra
            .charge_execution(InternalGas::new(120_000))
            .unwrap_err();
        assert_eq!(err.major_status(), StatusCode::EXECUTION_LIMIT_REACHED);
        assert_used(&algebra, 300_000, 0);
    }

    #[test]
    fn excess_out_of_gas_uses_remaining_balance() {
        let request = staking_request(400);
        let mut algebra = new_algebra(LATEST_GAS_FEATURE_VERSION, Some(&request));

        // Fits at 1x, but 91,000 above the base at 10x does not.
        let err = algebra
            .charge_execution(InternalGas::new(191_000))
            .unwrap_err();
        assert_eq!(err.major_status(), StatusCode::OUT_OF_GAS);
        assert_used(&algebra, 1_000_000, 0);
    }

    #[test]
    fn no_excess_pricing_without_staking_request_or_before_v1_51() {
        let mut algebra = new_algebra(LATEST_GAS_FEATURE_VERSION, None);
        let err = algebra
            .charge_execution(InternalGas::new(100_001))
            .unwrap_err();
        assert_eq!(err.major_status(), StatusCode::EXECUTION_LIMIT_REACHED);
        assert_used(&algebra, 100_001, 0);

        let request = staking_request(400);
        let mut algebra = new_algebra(gas_feature_versions::RELEASE_V1_50, Some(&request));
        assert!(algebra.charge_execution(InternalGas::new(120_000)).is_ok());
        assert_used(&algebra, 120_000, 0);
    }
}
