// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::traits::GasAlgebra;
use aptos_gas_algebra::{Fee, FeePerGasUnit, Gas, GasExpression, NumBytes, NumModules, Octa};
use aptos_gas_schedule::{gas_feature_versions, VMGasParameters};
use aptos_logger::error;
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

    max_execution_gas: InternalGas,
    execution_gas_used: InternalGas,

    max_io_gas: InternalGas,
    io_gas_used: InternalGas,

    max_storage_fee: Fee,
    // The gas consumed by the storage operations.
    storage_fee_in_internal_units: InternalGas,
    // The storage fee consumed by the storage operations.
    storage_fee_used: Fee,

    // The gas consumed by feature fees (e.g., randomness).
    feature_fee_in_internal_units: InternalGas,
    // The feature fee consumed.
    feature_fee_used: Fee,

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
        is_approved_gov_script: bool,
        balance: impl Into<Gas>,
        block_synchronization_kill_switch: &'a T,
    ) -> Self {
        let balance = balance.into().to_unit_with_params(&vm_gas_params.txn);

        let (max_execution_gas, max_io_gas, max_storage_fee) = if is_approved_gov_script
            && gas_feature_version >= gas_feature_versions::RELEASE_V1_13
        {
            (
                vm_gas_params.txn.max_execution_gas_gov,
                vm_gas_params.txn.max_io_gas_gov,
                vm_gas_params.txn.max_storage_fee_gov,
            )
        } else {
            (
                vm_gas_params.txn.max_execution_gas,
                vm_gas_params.txn.max_io_gas,
                vm_gas_params.txn.max_storage_fee,
            )
        };

        Self {
            feature_version: gas_feature_version,
            vm_gas_params,
            storage_gas_params,
            initial_balance: balance,
            balance,
            max_execution_gas,
            execution_gas_used: 0.into(),
            max_io_gas,
            io_gas_used: 0.into(),
            max_storage_fee,
            storage_fee_in_internal_units: 0.into(),
            storage_fee_used: 0.into(),
            feature_fee_in_internal_units: 0.into(),
            feature_fee_used: 0.into(),
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
            self.execution_gas_used + self.io_gas_used + self.storage_fee_in_internal_units + self.feature_fee_in_internal_units;
        if total != total_calculated {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "The per-category costs do not add up. {} (total) != {} = {} (exec) + {} (io) + {} (storage) + {} (feature)",
                        total,
                        total_calculated,
                        self.execution_gas_used,
                        self.io_gas_used,
                        self.storage_fee_in_internal_units,
                        self.feature_fee_in_internal_units,
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

        if self.feature_version >= 7 && self.execution_gas_used > self.max_execution_gas {
            Err(PartialVMError::new(StatusCode::EXECUTION_LIMIT_REACHED))
        } else {
            Ok(())
        }
    }

    fn charge_io(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> PartialVMResult<()> {
        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

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

        if self.feature_version >= 7 && self.io_gas_used > self.max_io_gas {
            Err(PartialVMError::new(StatusCode::IO_LIMIT_REACHED))
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

    fn charge_feature_fee(
        &mut self,
        abstract_amount: impl GasExpression<VMGasParameters, Unit = Octa>,
        gas_unit_price: FeePerGasUnit,
    ) -> PartialVMResult<()> {
        // Some tests use a unit price of 0. Skip charging to avoid division by zero,
        // consistent with how process_storage_fee_for_all guards charge_storage_fee.
        if gas_unit_price.is_zero() {
            return Ok(());
        }

        let amount = abstract_amount.evaluate(self.feature_version, &self.vm_gas_params);

        let txn_params = &self.vm_gas_params.txn;

        // Same Octa→internal-gas-unit conversion as charge_storage_fee.
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
                self.feature_fee_in_internal_units += gas_consumed_internal;
                self.feature_fee_used += amount;
            },
            None => {
                let old_balance = self.balance;
                self.balance = 0.into();
                // Match charge_storage_fee: only record partial accounting in v12+.
                // Always true when charge_feature_fee is reachable (gated on v48+),
                // but kept for consistency with the storage fee code path.
                if self.feature_version >= 12 {
                    self.feature_fee_in_internal_units += old_balance;
                    self.feature_fee_used += amount;
                }
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            },
        };

        Ok(())
    }

    fn feature_fee_used(&self) -> Fee {
        self.feature_fee_used
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
    use aptos_gas_schedule::{InitialGasSchedule, LATEST_GAS_FEATURE_VERSION, VMGasParameters};
    use aptos_vm_types::{
        resolver::NoopBlockSynchronizationKillSwitch,
        storage::StorageGasParameters,
    };

    fn make_algebra(
        balance_gas_units: u64,
    ) -> StandardGasAlgebra<'static, NoopBlockSynchronizationKillSwitch> {
        static KILL_SWITCH: NoopBlockSynchronizationKillSwitch =
            NoopBlockSynchronizationKillSwitch {};
        StandardGasAlgebra::new(
            LATEST_GAS_FEATURE_VERSION,
            VMGasParameters::initial(),
            StorageGasParameters::unlimited(),
            false,
            Gas::new(balance_gas_units),
            &KILL_SWITCH,
        )
    }

    #[test]
    fn charge_feature_fee_normal() {
        // 10_000 external gas units balance; gas_unit_price = 100 octas/gas-unit.
        let mut algebra = make_algebra(10_000);
        let gas_unit_price = FeePerGasUnit::new(100);

        // Charge 100_000 octas = 1000 gas units at price 100.
        let result = algebra.charge_feature_fee(Fee::new(100_000), gas_unit_price);
        assert!(result.is_ok());
        assert_eq!(u64::from(algebra.feature_fee_used()), 100_000);
        assert!(algebra.balance > InternalGas::zero());
    }

    #[test]
    fn charge_feature_fee_out_of_gas() {
        // 1 external gas unit balance; gas_unit_price = 100 octas/gas-unit.
        let mut algebra = make_algebra(1);
        let gas_unit_price = FeePerGasUnit::new(100);

        // Charge 100_000 octas = 1000 gas units, but only 1 available → OOG.
        let result = algebra.charge_feature_fee(Fee::new(100_000), gas_unit_price);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().major_status(),
            StatusCode::OUT_OF_GAS
        );
        assert_eq!(algebra.balance, InternalGas::zero());
        // The full fee is still recorded (for accounting), matching storage fee behavior.
        assert_eq!(u64::from(algebra.feature_fee_used()), 100_000);
    }

    #[test]
    fn charge_feature_fee_zero_gas_unit_price() {
        let mut algebra = make_algebra(10_000);
        let gas_unit_price = FeePerGasUnit::new(0);

        // Should return Ok and not charge anything (avoid division by zero).
        let result = algebra.charge_feature_fee(Fee::new(100_000), gas_unit_price);
        assert!(result.is_ok());
        assert_eq!(u64::from(algebra.feature_fee_used()), 0);
    }

    #[test]
    fn charge_feature_fee_not_in_block_gas_limit() {
        let mut algebra = make_algebra(10_000);
        let gas_unit_price = FeePerGasUnit::new(100);

        let balance_before = algebra.balance;
        algebra
            .charge_feature_fee(Fee::new(100_000), gas_unit_price)
            .unwrap();

        // Feature fee should not contribute to execution or IO gas.
        assert_eq!(algebra.execution_gas_used(), InternalGas::zero());
        assert_eq!(algebra.io_gas_used(), InternalGas::zero());
        // But should deduct from overall balance.
        assert!(algebra.balance < balance_before);
        // And the internal-unit counter should be non-zero.
        assert!(algebra.feature_fee_in_internal_units > InternalGas::zero());
    }
}
