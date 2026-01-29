// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{FINISH_WITH_CHUNKY_DKG_RESULT, RECONFIGURATION_WITH_DKG_MODULE},
    AptosVM,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    dkg::chunky_dkg::{CertifiedAggregatedChunkySubtranscript, ChunkyDKGState},
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    transaction::TransactionStatus,
    validator_verifier::ValidatorVerifier,
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
use serde::{Deserialize, Serialize};

#[derive(Debug)]
enum ExpectedFailure {
    // Move equivalent: `errors::invalid_argument(*)`
    EpochNotCurrent = 0x10201,
    MultiSigVerificationFailed = 0x010202,
    NotEnoughVotingPower = 0x010203,

    // Move equivalent: `errors::invalid_state(*)`
    MissingResourceChunkyDKGState = 0x30201,
    MissingResourceInprogressChunkyDKGSession = 0x30202,
    MissingResourceConfiguration = 0x30203,
    MissingResourceValidatorSet = 0x30204,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

/// Wrapper so that transcript bytes can be used with verify_multi_signatures (requires CryptoHash).
/// BCS(TranscriptBytesForSigning(bytes)) equals BCS(bytes), so the hash matches what was signed.
#[derive(Clone, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
struct TranscriptBytesForSigning(Vec<u8>);

impl AptosVM {
    pub(crate) fn process_chunky_dkg_result(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        transcript: CertifiedAggregatedChunkySubtranscript,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.process_chunky_dkg_result_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            transcript,
        ) {
            Ok((vm_status, vm_output)) => Ok((vm_status, vm_output)),
            Err(ExecutionFailure::Expected(failure)) => Ok((
                VMStatus::MoveAbort {
                    location: AbortLocation::Script,
                    code: failure as u64,
                    message: None,
                },
                VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
            )),
            Err(ExecutionFailure::Unexpected(vm_status)) => Err(vm_status),
        }
    }

    fn process_chunky_dkg_result_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        transcript: CertifiedAggregatedChunkySubtranscript,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let CertifiedAggregatedChunkySubtranscript {
            metadata,
            transcript_bytes,
            signature,
        } = transcript;

        let config_resource = ConfigurationResource::fetch_config(resolver).ok_or(
            ExecutionFailure::Expected(ExpectedFailure::MissingResourceConfiguration),
        )?;
        if metadata.epoch != config_resource.epoch() {
            return Err(ExecutionFailure::Expected(ExpectedFailure::EpochNotCurrent));
        }

        let validator_set = ValidatorSet::fetch_config(resolver).ok_or(
            ExecutionFailure::Expected(ExpectedFailure::MissingResourceValidatorSet),
        )?;
        let chunky_dkg_state = ChunkyDKGState::fetch_config(resolver).ok_or(
            ExecutionFailure::Expected(ExpectedFailure::MissingResourceChunkyDKGState),
        )?;

        let _in_progress_session_state =
            chunky_dkg_state
                .in_progress
                .as_ref()
                .ok_or(ExecutionFailure::Expected(
                    ExpectedFailure::MissingResourceInprogressChunkyDKGSession,
                ))?;

        let verifier = ValidatorVerifier::from(&validator_set);
        let authors = signature.get_signers_addresses(&verifier.get_ordered_account_addresses());

        // Check voting power.
        verifier
            .check_voting_power(authors.iter(), true)
            .map_err(|_| ExecutionFailure::Expected(ExpectedFailure::NotEnoughVotingPower))?;

        // Verify multi-sig (signature is over BCS(aggregated_subtranscript) = transcript_bytes).
        verifier
            .verify_multi_signatures(
                &TranscriptBytesForSigning(transcript_bytes.clone()),
                &signature,
            )
            .map_err(|_| ExecutionFailure::Expected(ExpectedFailure::MultiSigVerificationFailed))?;

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            transcript_bytes.as_move_value(),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &RECONFIGURATION_WITH_DKG_MODULE,
                FINISH_WITH_CHUNKY_DKG_RESULT,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(
                    e,
                    FINISH_WITH_CHUNKY_DKG_RESULT.as_str(),
                    log_context,
                )
            })
            .map_err(|r| ExecutionFailure::Unexpected(r.unwrap_err()))?;

        let output = get_system_transaction_output(
            session,
            module_storage,
            &self
                .storage_gas_params(log_context)
                .map_err(ExecutionFailure::Unexpected)?
                .change_set_configs,
        )
        .map_err(ExecutionFailure::Unexpected)?;

        Ok((VMStatus::Executed, output))
    }
}
