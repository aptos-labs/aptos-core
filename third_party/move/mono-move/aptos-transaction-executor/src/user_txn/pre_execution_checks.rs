// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Checks a transaction must pass before it runs. All but the account-creation
//! check need no state.
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
use aptos_types::on_chain_config::Features;
use aptos_vm_types::storage::space_pricing::DiskSpacePricing;

/// The execution gas units budgeted for creating the sender's account.
const ACCOUNT_CREATION_EXECUTION_GAS: u64 = 10;

pub(crate) struct PreExecutionChecker<'a> {
    gas_params: &'a AptosGasParameters,
    gas_feature_version: u64,
    features: &'a Features,
    txn_data: &'a TxnMetadata,
}

impl<'a> PreExecutionChecker<'a> {
    pub fn new(
        gas_params: &'a AptosGasParameters,
        gas_feature_version: u64,
        features: &'a Features,
        txn_data: &'a TxnMetadata,
    ) -> Self {
        Self {
            gas_params,
            gas_feature_version,
            features,
            txn_data,
        }
    }

    /// Runs the checks that need no state.
    pub fn run_checks(&self) -> Result<(), PreExecutionCheckFailure> {
        self.check_transaction_size()?;
        self.check_gas_price_bounds()?;
        self.check_gas_budget_upper_bound()?;
        self.check_gas_budget_covers_base_cost()?;
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

    /// When the sender's account is about to be created, the budget must also
    /// cover its storage slot on top of a minimal execution. Not enforced at a
    /// zero gas price.
    pub fn check_gas_budget_covers_account_creation(&self) -> Result<(), PreExecutionCheckFailure> {
        let gas_unit_price = u64::from(self.gas_price());
        if gas_unit_price == 0 {
            return Ok(());
        }
        let slot_fee = u64::from(
            DiskSpacePricing::new(self.gas_feature_version, self.features)
                .hack_estimated_fee_for_account_creation(self.txn_gas_params()),
        );
        let min_octas = gas_unit_price
            .saturating_mul(ACCOUNT_CREATION_EXECUTION_GAS)
            .saturating_add(slot_fee);
        let budget_octas = gas_unit_price.saturating_mul(u64::from(self.max_gas()));
        if budget_octas < min_octas {
            return Err(
                PreExecutionCheckFailure::GasBudgetBelowAccountCreationCost {
                    budget_octas,
                    min_octas,
                },
            );
        }
        Ok(())
    }
}
