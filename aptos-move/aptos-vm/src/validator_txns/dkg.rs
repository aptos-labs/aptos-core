// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    session::Session,
    system_module_names::{FINISH_WITH_DKG_RESULT, RECONFIGURATION_WITH_DKG_MODULE},
    validator_txns::dkg::{
        ExecutionFailure::{Expected, Unexpected},
        ExpectedFailure::*,
    },
    AptosVM,
};
use aptos_types::{
    dkg::{DKGState, DKGTrait, DKGTranscript, DefaultDKG},
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{ConfigurationResource, OnChainConfig},
    transaction::TransactionStatus,
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::{
    module_and_script_storage::module_storage::AptosModuleStorage, output::VMOutput,
};
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;

#[derive(Debug)]
enum ExpectedFailure {
    // Move equivalent: `errors::invalid_argument(*)`
    EpochNotCurrent = 0x10001,
    TranscriptDeserializationFailed = 0x10002,
    TranscriptVerificationFailed = 0x10003,

    // Move equivalent: `errors::invalid_state(*)`
    MissingResourceDKGState = 0x30001,
    MissingResourceInprogressDKGSession = 0x30002,
    MissingResourceConfiguration = 0x30003,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

impl AptosVM {
    pub(crate) fn process_dkg_result(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        dkg_transcript: DKGTranscript,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.process_dkg_result_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            dkg_transcript,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(Expected(failure)) => {
                // Pretend we are inside Move, and expected failures are like Move aborts.
                Ok((
                    VMStatus::MoveAbort(AbortLocation::Script, failure as u64),
                    VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
                ))
            },
            Err(Unexpected(vm_status)) => Err(vm_status),
        }
    }

    fn process_dkg_result_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        dkg_node: DKGTranscript,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let dkg_state =
            OnChainConfig::fetch_config(resolver).ok_or(Expected(MissingResourceDKGState))?;
        let config_resource = ConfigurationResource::fetch_config(resolver)
            .ok_or(Expected(MissingResourceConfiguration))?;
        let DKGState { in_progress, .. } = dkg_state;
        let in_progress_session_state =
            in_progress.ok_or(Expected(MissingResourceInprogressDKGSession))?;

        // Check epoch number.
        if dkg_node.metadata.epoch != config_resource.epoch() {
            return Err(Expected(EpochNotCurrent));
        }

        // Deserialize transcript and verify it.
        let pub_params = DefaultDKG::new_public_params(&in_progress_session_state.metadata);
        let transcript = bcs::from_bytes::<<DefaultDKG as DKGTrait>::Transcript>(
            dkg_node.transcript_bytes.as_slice(),
        )
        .map_err(|_| Expected(TranscriptDeserializationFailed))?;

        DefaultDKG::verify_transcript(&pub_params, &transcript)
            .map_err(|_| Expected(TranscriptVerificationFailed))?;

        // All check passed, invoke VM to publish DKG result on chain.
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            dkg_node.transcript_bytes.as_move_value(),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &RECONFIGURATION_WITH_DKG_MODULE,
                FINISH_WITH_DKG_RESULT,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, FINISH_WITH_DKG_RESULT.as_str(), log_context)
            })
            .map_err(|r| Unexpected(r.unwrap_err()))?;

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self
                .storage_gas_params(log_context)
                .map_err(Unexpected)?
                .change_set_configs,
        )
        .map_err(Unexpected)?;

        Ok((VMStatus::Executed, output))
    }
}
