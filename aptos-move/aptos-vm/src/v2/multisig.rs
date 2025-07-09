// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{
        FAILED_TRANSACTION_EXECUTION_CLEANUP, GET_NEXT_TRANSACTION_PAYLOAD,
        MULTISIG_ACCOUNT_MODULE, SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
    },
    v2::{
        loader::AptosLoader,
        session::{AptosSession, UserTransactionSession},
    },
};
use aptos_gas_meter::AptosGasMeter;
use aptos_types::transaction::{ExecutionError, MultisigTransactionPayload};
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::{StatusCode, VMStatus},
};
use move_vm_runtime::ScriptLoader;
use move_vm_types::gas::UnmeteredGasMeter;

impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: AptosLoader + ScriptLoader,
{
    /// First step in multisig transaction execution: extracts the payload to execute.
    pub(crate) fn extract_multisig_payload(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        multisig_address: AccountAddress,
    ) -> Result<(Vec<u8>, MultisigTransactionPayload), VMStatus> {
        // First, serialize the multisig payload.
        let provided_payload = self
            .executable
            .get_provided_payload_bytes(self.features())?;

        // Obtain the payload bytes by executing the Move function.
        let payload_bytes = self
            .session
            .execute_function_bypass_visibility(
                &MULTISIG_ACCOUNT_MODULE,
                GET_NEXT_TRANSACTION_PAYLOAD,
                vec![],
                serialize_values(&vec![
                    MoveValue::Address(multisig_address),
                    MoveValue::vector_u8(provided_payload),
                ]),
                gas_meter,
            )?
            .return_values
            .pop()
            .map(|(bytes, _)| bytes)
            .ok_or_else(|| {
                // We expect the payload to either exists on chain or be passed along with the
                // transaction.
                VMStatus::error(
                    StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                    Some("Multisig payload bytes not found".to_string()),
                )
            })?;

        // In order to obtain the payload, we need to deserialize the returned bytes twice:
        //   - First deserialization returns the actual bytes (vector<u8>) returned by the Move
        //     function.
        //   - Second deserialization returns the correct payload type.
        // If either deserialization fails for some reason, that means the user provided incorrect
        // payload data either during transaction creation or execution.
        let deserialization_error =
            |msg| VMStatus::error(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT, Some(msg));

        let payload_bytes = bcs::from_bytes::<Vec<u8>>(&payload_bytes).map_err(|err| {
            deserialization_error(format!(
                "Failed to deserialize multisig payload bytes: {err:?}"
            ))
        })?;
        let payload =
            bcs::from_bytes::<MultisigTransactionPayload>(&payload_bytes).map_err(|err| {
                deserialization_error(format!("Failed to deserialize multisig payload: {err:?}"))
            })?;

        Ok((payload_bytes, payload))
    }

    /// Second step in multisig execution: executes a payload.
    pub(crate) fn execute_multisig_payload(
        &mut self,
        gas_meter: &mut impl AptosGasMeter,
        payload: &MultisigTransactionPayload,
    ) -> Result<(), VMStatus> {
        match payload {
            MultisigTransactionPayload::EntryFunction(entry_func) => {
                self.execute_entry_function(gas_meter, entry_func)
            },
        }
    }

    /// Called ONLY WHEN multisig payload is executed successfully without any errors. Immediately
    /// followed by success transaction epilogue.
    pub(crate) fn execute_multisig_payload_success_hook(
        &mut self,
        multisig_address: AccountAddress,
        payload_bytes: Vec<u8>,
    ) -> Result<(), VMStatus> {
        let cleanup_args = self.default_multisig_cleanup_args(multisig_address, payload_bytes);
        self.session.execute_function_bypass_visibility(
            &MULTISIG_ACCOUNT_MODULE,
            SUCCESSFUL_TRANSACTION_EXECUTION_CLEANUP,
            vec![],
            cleanup_args,
            &mut UnmeteredGasMeter,
        )?;

        // Note: in the end, updated the extensions marking the start of transaction success
        // epilogue.
        self.update_extensions(SessionId::epilogue_meta(&self.txn_metadata));
        Ok(())
    }

    /// Called ONLY WHEN multisig payload failed its execution. Followed by success transaction
    /// epilogue, because multisig failure is recorded on-chain and transaction still succeeds.
    pub(crate) fn execute_multisig_payload_failure_hook(
        &mut self,
        multisig_address: AccountAddress,
        payload_bytes: Vec<u8>,
        failure_status: VMStatus,
    ) -> Result<(), VMStatus> {
        // Undo all changes made when executing the payload. Update the session ID so that the
        // extensions can resolve to new hash from epilogue. Finally, we save the new state so that
        // in case of failures, we can revert back to the underlying prologue state.
        self.undo_state_changes();
        self.update_extensions(SessionId::epilogue_meta(&self.txn_metadata));
        self.save_state_changes();

        let serialized_error = ExecutionError::try_from(failure_status)
            .map_err(|_| VMStatus::error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None))
            .and_then(|err| {
                bcs::to_bytes(&err).map_err(|_| {
                    VMStatus::error(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, None)
                })
            })?;
        let mut cleanup_args = self.default_multisig_cleanup_args(multisig_address, payload_bytes);
        cleanup_args.push(serialized_error);

        self.session.execute_function_bypass_visibility(
            &MULTISIG_ACCOUNT_MODULE,
            FAILED_TRANSACTION_EXECUTION_CLEANUP,
            vec![],
            cleanup_args,
            &mut UnmeteredGasMeter,
        )?;
        Ok(())
    }
}

// Private interfaces.
impl<'a, DataView, CodeLoader> UserTransactionSession<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: AptosLoader + ScriptLoader,
{
    /// Returns default arguments for multisig epilogue that runs after the multisig payload is
    /// executed.
    fn default_multisig_cleanup_args(
        &self,
        multisig_address: AccountAddress,
        payload_bytes: Vec<u8>,
    ) -> Vec<Vec<u8>> {
        serialize_values(&vec![
            MoveValue::Address(self.txn_metadata.sender),
            MoveValue::Address(multisig_address),
            MoveValue::vector_u8(payload_bytes),
        ])
    }
}
