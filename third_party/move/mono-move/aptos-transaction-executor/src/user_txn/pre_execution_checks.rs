// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks a transaction must pass before execution touches any state.
//!
//! TODO(completeness): currently this uses the legacy VM types (e.g. `AptosGasParameters`),
//! but eventually should switch to a new on-chain config format.

use super::metadata::TxnMetadata;
use crate::errors::PreExecutionCheckFailure;
use aptos_gas_algebra::{
    FeePerGasUnit, Gas, GasExpression, InternalGas, InternalGasUnit, NumBytes,
};
use aptos_gas_schedule::{
    gas_params::txn::{
        ENCRYPTED_TXN_DECRYPTION_BASE_COST, KEYLESS_BASE_COST, SLH_DSA_SHA2_128S_BASE_COST,
    },
    AptosGasParameters, TransactionGasParameters, VMGasParameters,
};
use aptos_types::on_chain_config::ApprovedExecutionHashes;

/// The range a requested limits multiplier must fall in, in percent, where 100
/// is 1x. Must match the Move constants in `0x1::transaction_limits`.
const MIN_MULTIPLIER_PERCENT: u64 = 100;
const MAX_MULTIPLIER_PERCENT: u64 = 10_000;

pub(crate) struct PreExecutionChecker<'a> {
    gas_params: &'a AptosGasParameters,
    gas_feature_version: u64,
    txn_data: &'a TxnMetadata,
    /// Whether the script is one governance has approved.
    is_approved_gov_script: bool,
}

impl<'a> PreExecutionChecker<'a> {
    pub fn new(
        gas_params: &'a AptosGasParameters,
        gas_feature_version: u64,
        approved_gov_scripts: Option<&ApprovedExecutionHashes>,
        txn_data: &'a TxnMetadata,
    ) -> Self {
        // The hash is empty for everything but a script, so an entry function
        // can never match one.
        let is_approved_gov_script = !txn_data.script_hash.is_empty()
            && approved_gov_scripts
                .is_some_and(|approved| approved.contains_script_hash(&txn_data.script_hash));
        Self {
            gas_params,
            gas_feature_version,
            txn_data,
            is_approved_gov_script,
        }
    }

    pub fn run_checks(&self) -> Result<(), PreExecutionCheckFailure> {
        self.check_limits_multipliers()?;
        self.check_transaction_size()?;
        self.check_gas_price_bounds()?;
        self.check_gas_budget_upper_bound()?;
        self.check_gas_budget_covers_base_cost()?;
        // TODO(completeness, metering): the account-creation affordability
        // check, once lazy account creation is supported.
        // TODO(security, completeness): the authenticator feature gates
        // (`SingleSender`, WebAuthn, SLH-DSA) are not enforced, so an
        // authenticator governance has disabled still executes.
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

    /// `cost` if the surcharge applies, otherwise zero.
    fn surcharge(
        &self,
        applies: bool,
        cost: impl GasExpression<VMGasParameters, Unit = InternalGasUnit>,
    ) -> InternalGas {
        if applies {
            cost.evaluate(self.gas_feature_version, &self.gas_params.vm)
        } else {
            InternalGas::zero()
        }
    }

    /// Checks a request for raised limits before the prologue sees it.
    /// - An approved governance script may not make one.
    /// - Each multiplier must be above 1x and at most the cap.
    /// The staking behind the request is checked by the prologue.
    //
    // The prologue also checks the range, but the gas meter is built from the
    // multipliers before the prologue runs, so they must be sane by then. The
    // governance rule exists only here: the framework does not know which
    // scripts are approved.
    fn check_limits_multipliers(&self) -> Result<(), PreExecutionCheckFailure> {
        let Some(request) = &self.txn_data.txn_limits_request else {
            return Ok(());
        };
        if self.is_approved_gov_script {
            return Err(PreExecutionCheckFailure::LimitsRequestOnGovernanceScript);
        }
        let multipliers = request.multipliers();
        for percent in [
            multipliers.execution_multiplier_percent(),
            multipliers.io_multiplier_percent(),
        ] {
            if percent <= MIN_MULTIPLIER_PERCENT || MAX_MULTIPLIER_PERCENT < percent {
                return Err(PreExecutionCheckFailure::InvalidLimitsMultiplier {
                    percent,
                    min: MIN_MULTIPLIER_PERCENT,
                    max: MAX_MULTIPLIER_PERCENT,
                });
            }
        }
        Ok(())
    }

    /// Checks if the transaction size is within the allowed maximum. A script
    /// governance approved gets a larger allowance.
    fn check_transaction_size(&self) -> Result<(), PreExecutionCheckFailure> {
        let params = self.txn_gas_params();
        let max = if self.is_approved_gov_script {
            params.max_transaction_size_in_bytes_gov
        } else {
            params.max_transaction_size_in_bytes
        };
        if self.txn_size() > max {
            return Err(PreExecutionCheckFailure::TransactionTooLarge {
                size: self.txn_size().into(),
                max: max.into(),
            });
        }
        Ok(())
    }

    /// Checks if the gas unit price is within the allowed global minimum and maximum.
    /// An encrypted transaction has its own, higher minimum.
    fn check_gas_price_bounds(&self) -> Result<(), PreExecutionCheckFailure> {
        let min = self.txn_gas_params().min_price_per_gas_unit;
        if self.gas_price() < min {
            return Err(PreExecutionCheckFailure::GasPriceBelowMinimum {
                price: self.gas_price().into(),
                min: min.into(),
            });
        }
        if self.txn_data.is_encrypted_txn {
            let encrypted_min = min.max(self.txn_gas_params().encrypted_txn_min_price_per_gas_unit);
            if self.gas_price() < encrypted_min {
                return Err(PreExecutionCheckFailure::EncryptedGasPriceBelowMinimum {
                    price: self.gas_price().into(),
                    min: encrypted_min.into(),
                });
            }
        }
        // A request for raised limits has its own, higher price floor.
        if self.txn_data.txn_limits_request.is_some() {
            let high_limit_min = self.txn_gas_params().high_limit_txn_min_price_per_gas_unit;
            if self.gas_price() < high_limit_min {
                return Err(PreExecutionCheckFailure::HighLimitGasPriceBelowMinimum {
                    price: self.gas_price().into(),
                    min: high_limit_min.into(),
                });
            }
        }
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

    /// The budget must at least cover the transaction's base cost: its
    /// intrinsic cost plus any authentication and decryption surcharges.
    // TODO(metering): deriving the base cost from the existing gas parameters
    // is temporary; revisit with the gas schedule and VM config design.
    fn check_gas_budget_covers_base_cost(&self) -> Result<(), PreExecutionCheckFailure> {
        let keyless = self.surcharge(self.txn_data.is_keyless, KEYLESS_BASE_COST);
        let slh_dsa = self.surcharge(self.txn_data.is_slh_dsa, SLH_DSA_SHA2_128S_BASE_COST);
        let decryption = self.surcharge(
            self.txn_data.is_encrypted_txn,
            ENCRYPTED_TXN_DECRYPTION_BASE_COST,
        );
        let intrinsic = self
            .txn_gas_params()
            .calculate_intrinsic_gas(self.txn_size())
            .evaluate(self.gas_feature_version, &self.gas_params.vm);
        let base_cost: Gas = (intrinsic + keyless + slh_dsa + decryption)
            .to_unit_round_up_with_params(self.txn_gas_params());
        if self.max_gas() < base_cost {
            return Err(PreExecutionCheckFailure::GasBudgetBelowIntrinsicCost {
                max_gas: self.max_gas().into(),
                min: base_cost.into(),
            });
        }
        Ok(())
    }
}
