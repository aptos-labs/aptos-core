// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::create_account_if_does_not_exist,
    errors::expect_only_successful_execution,
    session::Session,
    system_module_names::{ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST},
    transaction_metadata::TransactionMetadata,
    AptosVM,
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::fee_statement::FeeStatement;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::module_and_script_storage::module_storage::AptosModuleStorage;
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::{
    account_address::AccountAddress,
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::{logging::expect_no_verification_errors, module_traversal::TraversalContext};
use move_vm_types::gas::UnmeteredGasMeter;

pub(crate) fn abort_hook_try_create_account(
    session: &mut impl Session,
    sender: AccountAddress,
    gas_meter: &mut impl AptosGasMeter,
    traversal_context: &mut TraversalContext,
    module_storage: &impl AptosModuleStorage,
    log_context: &AdapterLogSchema,
) -> Result<(), VMStatus> {
    create_account_if_does_not_exist(
        session,
        module_storage,
        gas_meter,
        sender,
        traversal_context,
    )
    .or_else(|_| {
        // If this fails, it is likely due to out of gas, so we try again without
        // metering and then validate below that we charged sufficiently.
        create_account_if_does_not_exist(
            session,
            module_storage,
            &mut UnmeteredGasMeter,
            sender,
            traversal_context,
        )
    })
    .map_err(expect_no_verification_errors)
    .or_else(|err| {
        expect_only_successful_execution(
            err,
            &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
            log_context,
        )
    })?;
    Ok(())
}

pub(crate) fn abort_hook_verify_gas_charge_for_slot_creation(
    vm: &AptosVM,
    txn_metadata: &TransactionMetadata,
    log_context: &AdapterLogSchema,
    gas_meter: &mut impl AptosGasMeter,
    fee_statement: &FeeStatement,
) -> Result<(), VMStatus> {
    let gas_params = vm.gas_params(log_context)?;
    let gas_unit_price = u64::from(txn_metadata.gas_unit_price());
    if gas_unit_price != 0 || !vm.features().is_default_account_resource_enabled() {
        let gas_used = fee_statement.gas_used();
        let storage_fee = fee_statement.storage_fee_used();
        let storage_refund = fee_statement.storage_fee_refund();

        let actual = gas_used * gas_unit_price + storage_fee - storage_refund;
        let expected = u64::from(
            gas_meter
                .disk_space_pricing()
                .hack_account_creation_fee_lower_bound(&gas_params.vm.txn),
        );
        if actual < expected {
            expect_only_successful_execution(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message(
                        "Insufficient fee for storing account for lazy account creation"
                            .to_string(),
                    )
                    .finish(Location::Undefined),
                &format!("{:?}::{}", ACCOUNT_MODULE, CREATE_ACCOUNT_IF_DOES_NOT_EXIST),
                log_context,
            )?;
        }
    }

    Ok(())
}
