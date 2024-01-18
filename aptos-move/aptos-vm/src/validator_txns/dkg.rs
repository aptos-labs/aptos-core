// Copyright © Aptos Foundation

use crate::{
    aptos_vm::{get_or_vm_startup_failure, load_on_chain_config_from_resolver},
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{FINISH_WITH_DKG_RESULT, RECONFIGURATION_WITH_DKG_MODULE},
    validator_txns::dkg::{
        ExecutionFailure::{Expected, Unexpected},
        ExpectedFailure::*,
    },
    AptosVM,
};
use aptos_types::{
    dkg::{DKGNode, DKGState, DKGTrait, DummyDKG},
    fee_statement::FeeStatement,
    move_utils::as_move_value::AsMoveValue,
    transaction::{ExecutionStatus, TransactionStatus},
};
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_types::gas::UnmeteredGasMeter;

enum ExpectedFailure {
    MissingResourceDKGState = 0,
    MissingResourceInprogressDKGSession,
    EpochNotCurrent,
    TranscriptDeserializationFailed,
    TranscriptVerificationFailed,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

impl AptosVM {
    pub(crate) fn process_dkg_result(
        &self,
        resolver: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        dkg_node: DKGNode,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.process_dkg_result_inner(resolver, log_context, session_id, dkg_node) {
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
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        dkg_node: DKGNode,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let dkg_state = load_on_chain_config_from_resolver::<DKGState>(resolver)
            .map_err(|e| {
                let internal_err = VMStatus::error(
                    StatusCode::ABORTED,
                    Some(format!(
                        "process_dkg_result failed with dkg state loading error: {e}"
                    )),
                );
                Unexpected(internal_err)
            })?
            .ok_or_else(|| Expected(MissingResourceDKGState))?;

        let DKGState { in_progress, .. } = dkg_state;
        let in_progress_session_state =
            in_progress.ok_or_else(|| Expected(MissingResourceInprogressDKGSession))?;

        // Check epoch number.
        if dkg_node.metadata.epoch != in_progress_session_state.metadata.dealer_epoch {
            return Err(Expected(EpochNotCurrent));
        }

        // Deserialize transcript and verify it.
        let pub_params = DummyDKG::new_public_params(&in_progress_session_state.metadata);
        let transcript = DummyDKG::deserialize_transcript(dkg_node.transcript_bytes.as_slice())
            .map_err(|_| Expected(TranscriptDeserializationFailed))?;

        DummyDKG::verify_transcript(&pub_params, &transcript)
            .map_err(|_| Expected(TranscriptVerificationFailed))?;

        // All check passed, invoke VM to publish DKG result on chain.
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            dkg_node.transcript_bytes.as_move_value(),
        ];

        session
            .execute_function_bypass_visibility(
                &RECONFIGURATION_WITH_DKG_MODULE,
                FINISH_WITH_DKG_RESULT,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, FINISH_WITH_DKG_RESULT.as_str(), log_context)
            })
            .map_err(|r| Unexpected(r.unwrap_err()))?;

        let output = crate::aptos_vm::get_transaction_output(
            session,
            FeeStatement::zero(),
            ExecutionStatus::Success,
            &get_or_vm_startup_failure(&self.storage_gas_params, log_context)
                .map_err(Unexpected)?
                .change_set_configs,
        )
        .map_err(Unexpected)?;

        Ok((VMStatus::Executed, output))
    }
}
