// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks a transaction must pass before execution touches any state.
//!
//! TODO(completeness): currently this uses the legacy VM types (e.g. `AptosGasParameters`),
//! but eventually should switch to a new on-chain config format.

use super::metadata::TxnMetadata;
use crate::errors::PreExecutionCheckFailure;
use aptos_gas_algebra::{FeePerGasUnit, Gas, GasExpression, InternalGas, NumBytes};
use aptos_gas_schedule::{
    gas_params::txn::{KEYLESS_BASE_COST, SLH_DSA_SHA2_128S_BASE_COST},
    AptosGasParameters, TransactionGasParameters,
};

pub(crate) struct PreExecutionChecker<'a> {
    gas_params: &'a AptosGasParameters,
    gas_feature_version: u64,
    txn_data: &'a TxnMetadata,
}

impl<'a> PreExecutionChecker<'a> {
    pub fn new(
        gas_params: &'a AptosGasParameters,
        gas_feature_version: u64,
        txn_data: &'a TxnMetadata,
    ) -> Self {
        Self {
            gas_params,
            gas_feature_version,
            txn_data,
        }
    }

    pub fn run_checks(&self) -> Result<(), PreExecutionCheckFailure> {
        self.check_transaction_size()?;
        self.check_gas_price_bounds()?;
        self.check_gas_budget_upper_bound()?;
        self.check_gas_budget_covers_base_costs()?;
        // TODO(completeness, metering): the account-creation affordability
        // check, once lazy account creation is supported.
        Ok(())
    }

    fn txn_gas_params(&self) -> &TransactionGasParameters {
        &self.gas_params.vm.txn
    }

    fn txn_size(&self) -> NumBytes {
        self.txn_data.transaction_size.into()
    }

    fn max_gas(&self) -> Gas {
        self.txn_data.max_gas_amount.into()
    }

    fn gas_price(&self) -> FeePerGasUnit {
        self.txn_data.gas_unit_price.into()
    }

    /// Checks if the transaction size is within the allowed maximum.
    // TODO(completeness): approved governance scripts get a larger size
    // allowance (`max_transaction_size_in_bytes_gov`); revisit with scripts.
    fn check_transaction_size(&self) -> Result<(), PreExecutionCheckFailure> {
        let max = self.txn_gas_params().max_transaction_size_in_bytes;
        if self.txn_size() > max {
            return Err(PreExecutionCheckFailure::TransactionTooLarge {
                size: self.txn_size().into(),
                max: max.into(),
            });
        }
        Ok(())
    }

    /// Checks if the gas unit price is within the allowed global minimum and maximum.
    fn check_gas_price_bounds(&self) -> Result<(), PreExecutionCheckFailure> {
        let min = self.txn_gas_params().min_price_per_gas_unit;
        if self.gas_price() < min {
            return Err(PreExecutionCheckFailure::GasPriceBelowMinimum {
                price: self.gas_price().into(),
                min: min.into(),
            });
        }
        // TODO(completeness): the staking high-limit minimum price, once
        // transaction-limits requests are supported.
        let max = self.txn_gas_params().max_price_per_gas_unit;
        if self.gas_price() > max {
            return Err(PreExecutionCheckFailure::GasPriceAboveMaximum {
                price: self.gas_price().into(),
                max: max.into(),
            });
        }
        Ok(())
    }

    /// Checks if the gas budget of the transaction is within the global maximum.
    fn check_gas_budget_upper_bound(&self) -> Result<(), PreExecutionCheckFailure> {
        let bound = self.txn_gas_params().maximum_number_of_gas_units;
        if self.max_gas() > bound {
            return Err(PreExecutionCheckFailure::GasBudgetAboveBound {
                max_gas: self.max_gas().into(),
                bound: bound.into(),
            });
        }
        Ok(())
    }

    /// The budget must at least cover the transaction's intrinsic cost plus
    /// any authentication surcharges.
    fn check_gas_budget_covers_base_costs(&self) -> Result<(), PreExecutionCheckFailure> {
        let keyless = if self.txn_data.is_keyless {
            KEYLESS_BASE_COST.evaluate(self.gas_feature_version, &self.gas_params.vm)
        } else {
            InternalGas::zero()
        };
        let slh_dsa = if self.txn_data.is_slh_dsa {
            SLH_DSA_SHA2_128S_BASE_COST.evaluate(self.gas_feature_version, &self.gas_params.vm)
        } else {
            InternalGas::zero()
        };
        // TODO(completeness): the encrypted-transaction decryption surcharge
        // and minimum price, once encrypted payloads are supported.
        let intrinsic = self
            .txn_gas_params()
            .calculate_intrinsic_gas(self.txn_size())
            .evaluate(self.gas_feature_version, &self.gas_params.vm);
        let min_gas: Gas =
            (intrinsic + keyless + slh_dsa).to_unit_round_up_with_params(self.txn_gas_params());
        if self.max_gas() < min_gas {
            return Err(PreExecutionCheckFailure::GasBudgetBelowIntrinsicCost {
                max_gas: self.max_gas().into(),
                min: min_gas.into(),
            });
        }
        Ok(())
    }
}
