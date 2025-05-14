// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{move_vm_ext::AptosMoveResolver, transaction_metadata::TransactionMetadata};
use aptos_gas_algebra::{FeePerGasUnit, Gas, GasExpression, InternalGas};
use aptos_gas_meter::{StandardGasAlgebra, StandardGasMeter};
use aptos_gas_schedule::{
    gas_feature_versions::RELEASE_V1_13, gas_params::txn::KEYLESS_BASE_COST, AptosGasParameters,
    FromOnChainGasSchedule, VMGasParameters,
};
use aptos_logger::{enabled, Level};
use aptos_memory_usage_tracker::MemoryTrackedGasMeter;
use aptos_types::on_chain_config::{
    ConfigStorage, Features, GasSchedule, GasScheduleV2, OnChainConfig,
};
use aptos_vm_logging::{log_schema::AdapterLogSchema, speculative_log, speculative_warn};
use aptos_vm_types::storage::{
    io_pricing::IoPricing, space_pricing::DiskSpacePricing, StorageGasParameters,
};
use move_core_types::{
    gas_algebra::NumArgs,
    vm_status::{StatusCode, VMStatus},
};
use move_core_types::gas_algebra::NumBytes;

/// This is used until gas version 18, which introduces a configurable entry for this.
const MAXIMUM_APPROVED_TRANSACTION_SIZE_LEGACY: u64 = 1024 * 1024;

pub fn get_gas_config_from_storage(
    config_storage: &impl ConfigStorage,
) -> (Result<AptosGasParameters, String>, u64) {
    match GasScheduleV2::fetch_config(config_storage) {
        Some(gas_schedule) => {
            let feature_version = gas_schedule.feature_version;
            let map = gas_schedule.into_btree_map();
            (
                AptosGasParameters::from_on_chain_gas_schedule(&map, feature_version),
                feature_version,
            )
        },
        None => match GasSchedule::fetch_config(config_storage) {
            Some(gas_schedule) => {
                let map = gas_schedule.into_btree_map();
                (AptosGasParameters::from_on_chain_gas_schedule(&map, 0), 0)
            },
            None => (Err("Neither gas schedule v2 nor v1 exists.".to_string()), 0),
        },
    }
}

pub fn get_gas_parameters(
    features: &Features,
    config_storage: &impl ConfigStorage,
) -> (
    Result<AptosGasParameters, String>,
    Result<StorageGasParameters, String>,
    u64,
) {
    let (mut gas_params, gas_feature_version) = get_gas_config_from_storage(config_storage);

    let storage_gas_params = match &mut gas_params {
        Ok(gas_params) => {
            let storage_gas_params =
                StorageGasParameters::new(gas_feature_version, features, gas_params, config_storage);

            // TODO(gas): Table extension utilizes IoPricing directly.
            // Overwrite table io gas parameters with global io pricing.
            let g = &mut gas_params.natives.table;
            match gas_feature_version {
                0..=1 => (),
                2..=6 => {
                    if let IoPricing::V2(pricing) = &storage_gas_params.io_pricing {
                        g.common_load_base_legacy = pricing.per_item_read * NumArgs::new(1);
                        g.common_load_base_new = 0.into();
                        g.common_load_per_byte = pricing.per_byte_read;
                        g.common_load_failure = 0.into();
                    }
                }
                7..=9 => {
                    if let IoPricing::V2(pricing) = &storage_gas_params.io_pricing {
                        g.common_load_base_legacy = 0.into();
                        g.common_load_base_new = pricing.per_item_read * NumArgs::new(1);
                        g.common_load_per_byte = pricing.per_byte_read;
                        g.common_load_failure = 0.into();
                    }
                }
                10.. => {
                    g.common_load_base_legacy = 0.into();
                    g.common_load_base_new = gas_params.vm.txn.storage_io_per_state_slot_read * NumArgs::new(1);
                    g.common_load_per_byte = gas_params.vm.txn.storage_io_per_state_byte_read;
                    g.common_load_failure = 0.into();
                }
            };
            Ok(storage_gas_params)
        },
        Err(err) => Err(format!("Failed to initialize storage gas params due to failure to load main gas parameters: {}", err)),
    };

    (gas_params, storage_gas_params, gas_feature_version)
}

/// Gas meter used in the production (validator) setup.
pub type ProdGasMeter = MemoryTrackedGasMeter<StandardGasMeter<StandardGasAlgebra>>;

/// Creates a gas meter intended for executing transactions in the production.
///
/// The current setup consists of the standard gas meter & algebra + the memory usage tracker.
pub fn make_prod_gas_meter(
    gas_feature_version: u64,
    vm_gas_params: VMGasParameters,
    storage_gas_params: StorageGasParameters,
    is_approved_gov_script: bool,
    meter_balance: Gas,
) -> ProdGasMeter {
    MemoryTrackedGasMeter::new(StandardGasMeter::new(StandardGasAlgebra::new(
        gas_feature_version,
        vm_gas_params,
        storage_gas_params,
        is_approved_gov_script,
        meter_balance,
    )))
}

/// Invariants facilitating gas checks of the transactions.
pub (crate) struct TransactionGasCheckInvariants {
    pub(crate) gas_unit_price: FeePerGasUnit,
    pub(crate) max_gas_amount: Gas,
    pub(crate) transaction_size: NumBytes,
    pub(crate) script_size: NumBytes,
    pub(crate) is_keyless: bool,
    pub(crate) is_account_init_for_sponsored_transaction: bool,
}

pub(crate) fn check_gas(
    gas_params: &AptosGasParameters,
    gas_feature_version: u64,
    resolver: &impl AptosMoveResolver,
    txn_metadata: &TransactionMetadata,
    features: &Features,
    is_approved_gov_script: bool,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let txn_gas_metadata = TransactionGasCheckInvariants {
        gas_unit_price: txn_metadata.gas_unit_price(),
        max_gas_amount: txn_metadata.max_gas_amount(),
        transaction_size: txn_metadata.transaction_size,
        script_size: txn_metadata.script_size,
        is_keyless: txn_metadata.is_keyless(),
        is_account_init_for_sponsored_transaction: crate::aptos_vm::is_account_init_for_sponsored_transaction(txn_metadata, features, resolver)?,
    };
    check_gas_for_parameters(gas_params, gas_feature_version, features, txn_gas_metadata, is_approved_gov_script, log_context)
}

pub(crate) fn check_gas_for_parameters(
    gas_params: &AptosGasParameters,
    gas_feature_version: u64,
    features: &Features,
    txn_gas_metadata: TransactionGasCheckInvariants,
    is_approved_gov_script: bool,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    let txn_gas_params = &gas_params.vm.txn;
    let raw_bytes_len = txn_gas_metadata.transaction_size;

    if is_approved_gov_script {
        let max_txn_size_gov = if gas_feature_version >= RELEASE_V1_13 {
            gas_params.vm.txn.max_transaction_size_in_bytes_gov
        } else {
            MAXIMUM_APPROVED_TRANSACTION_SIZE_LEGACY.into()
        };

        if txn_gas_metadata.transaction_size > max_txn_size_gov
            // Ensure that it is only the approved payload that exceeds the
            // maximum. The (unknown) user input should be restricted to the original
            // maximum transaction size.
            || txn_gas_metadata.transaction_size
            > txn_gas_metadata.script_size + txn_gas_params.max_transaction_size_in_bytes
        {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Governance transaction size too big {} payload size {}",
                    txn_gas_metadata.transaction_size, txn_gas_metadata.script_size,
                ),
            );
            return Err(VMStatus::error(
                StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE,
                None,
            ));
        }
    } else if txn_gas_metadata.transaction_size > txn_gas_params.max_transaction_size_in_bytes {
        speculative_warn!(
            log_context,
            format!(
                "[VM] Transaction size too big {} (max {})",
                txn_gas_metadata.transaction_size, txn_gas_params.max_transaction_size_in_bytes
            ),
        );
        return Err(VMStatus::error(
            StatusCode::EXCEEDED_MAX_TRANSACTION_SIZE,
            None,
        ));
    }

    // The submitted max gas units that the transaction can consume is greater than the
    // maximum number of gas units bound that we have set for any
    // transaction.
    if txn_gas_metadata.max_gas_amount > txn_gas_params.maximum_number_of_gas_units {
        speculative_warn!(
            log_context,
            format!(
                "[VM] Gas unit error; max {}, submitted {}",
                txn_gas_params.maximum_number_of_gas_units,
                txn_gas_metadata.max_gas_amount
            ),
        );
        return Err(VMStatus::error(
            StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
            None,
        ));
    }

    // The submitted transactions max gas units needs to be at least enough to cover the
    // intrinsic cost of the transaction as calculated against the size of the
    // underlying `RawTransaction`.
    let keyless = if txn_gas_metadata.is_keyless {
        KEYLESS_BASE_COST.evaluate(gas_feature_version, &gas_params.vm)
    } else {
        InternalGas::zero()
    };
    let intrinsic_gas = txn_gas_params
        .calculate_intrinsic_gas(raw_bytes_len)
        .evaluate(gas_feature_version, &gas_params.vm);
    let total_rounded: Gas = (intrinsic_gas + keyless).to_unit_round_up_with_params(txn_gas_params);

    if txn_gas_metadata.max_gas_amount < total_rounded {
        speculative_warn!(
            log_context,
            format!(
                "[VM] Gas unit error; min {}, submitted {}",
                total_rounded,
                txn_gas_metadata.max_gas_amount
            ),
        );
        return Err(VMStatus::error(
            StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
            None,
        ));
    }

    // The submitted gas price is less than the minimum gas unit price set by the VM.
    // NB: MIN_PRICE_PER_GAS_UNIT may equal zero, but need not in the future. Hence why
    // we turn off the clippy warning.
    #[allow(clippy::absurd_extreme_comparisons)]
    let below_min_bound = txn_gas_metadata.gas_unit_price < txn_gas_params.min_price_per_gas_unit;
    if below_min_bound {
        speculative_warn!(
            log_context,
            format!(
                "[VM] Gas unit error; min {}, submitted {}",
                txn_gas_params.min_price_per_gas_unit,
                txn_gas_metadata.gas_unit_price
            ),
        );
        return Err(VMStatus::error(
            StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND,
            None,
        ));
    }

    // The submitted gas price is greater than the maximum gas unit price set by the VM.
    if txn_gas_metadata.gas_unit_price > txn_gas_params.max_price_per_gas_unit {
        speculative_warn!(
            log_context,
            format!(
                "[VM] Gas unit error; min {}, submitted {}",
                txn_gas_params.max_price_per_gas_unit,
                txn_gas_metadata.gas_unit_price
            ),
        );
        return Err(VMStatus::error(
            StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND,
            None,
        ));
    }

    // If this is a sponsored transaction for a potentially new account, ensure there's enough
    // gas to cover storage, execution, and IO costs.
    // TODO: This isn't the cleaning code, thus we localize it just here and will remove it
    // once accountv2 is available and we no longer need to create accounts.
    if  txn_gas_metadata.is_account_init_for_sponsored_transaction {
        let gas_unit_price: u64 = txn_gas_metadata.gas_unit_price.into();
        let max_gas_amount: u64 = txn_gas_metadata.max_gas_amount.into();
        let pricing = DiskSpacePricing::new(gas_feature_version, features);
        let storage_fee_per_account_create: u64 = pricing
            .hack_estimated_fee_for_account_creation(txn_gas_params)
            .into();

        let expected = gas_unit_price * 10 + 2 * storage_fee_per_account_create;
        let actual = gas_unit_price * max_gas_amount;

        if actual < expected {
            speculative_warn!(
                log_context,
                format!(
                    "[VM] Insufficient gas for sponsored transaction; min {}, submitted {}",
                    expected, actual,
                ),
            );
            return Err(VMStatus::error(
                StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
                None,
            ));
        }
    }
    Ok(())
}
