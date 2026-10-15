// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    aptos_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{AptosMoveResolver, SessionId},
    system_module_names::{TRUST_OBSERVATION_MODULE, UPSERT_OBSERVED_VALUE},
    AptosVM,
};
use aptos_logger::debug;
use aptos_types::{
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{ConfigurationResource, OnChainConfig, ValidatorSet},
    transaction::TransactionStatus,
    trust_observation::{Config, ObservedTrustData, QuorumCertifiedUpdate, MAX_VALUE_BYTES},
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

#[derive(Debug)]
enum ExpectedFailure {
    MissingValidatorSet = 0x30101,
    MissingConfig = 0x30102,
    MissingObservedTrustData = 0x30103,
    MissingConfigurationResource = 0x30104,
    UnknownTarget = 0x010101,
    IncorrectConfigVersion = 0x010102,
    IncorrectVersion = 0x010103,
    InvalidValue = 0x010104,
    NotEnoughVotingPower = 0x010105,
    MultiSigVerificationFailed = 0x010106,
    IncorrectEpoch = 0x010107,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

use ExecutionFailure::{Expected, Unexpected};

impl AptosVM {
    pub(crate) fn process_observed_trust_update(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        debug!("Processing observed trust update");
        match self.process_observed_trust_update_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            update,
        ) {
            Ok(output) => Ok(output),
            Err(Expected(failure)) => Ok((
                VMStatus::MoveAbort {
                    location: AbortLocation::Script,
                    code: failure as u64,
                    message: None,
                },
                VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
            )),
            Err(Unexpected(status)) => Err(status),
        }
    }

    fn process_observed_trust_update_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        module_storage: &impl AptosModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        let validator_set = ValidatorSet::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(Expected(ExpectedFailure::MissingValidatorSet))?;
        let config = Config::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(Expected(ExpectedFailure::MissingConfig))?;
        let observed_state = ObservedTrustData::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(Expected(ExpectedFailure::MissingObservedTrustData))?;
        let configuration = ConfigurationResource::fetch_config(resolver)
            .ok()
            .flatten()
            .ok_or(Expected(ExpectedFailure::MissingConfigurationResource))?;

        let QuorumCertifiedUpdate {
            update: observed,
            multi_sig,
        } = update;
        let target = config
            .get(&observed.target_id)
            .ok_or(Expected(ExpectedFailure::UnknownTarget))?;
        if target.config_version != observed.config_version {
            return Err(Expected(ExpectedFailure::IncorrectConfigVersion));
        }
        if observed.epoch != configuration.epoch() {
            return Err(Expected(ExpectedFailure::IncorrectEpoch));
        }
        if observed.value.is_empty()
            || observed.value.len() > MAX_VALUE_BYTES
            || u64::try_from(observed.value.len()).unwrap_or(u64::MAX) > target.max_bytes
        {
            return Err(Expected(ExpectedFailure::InvalidValue));
        }
        let current_version = observed_state
            .get(&observed.target_id)
            .map_or(0, |entry| entry.version);
        if current_version.checked_add(1) != Some(observed.version) {
            return Err(Expected(ExpectedFailure::IncorrectVersion));
        }

        let verifier = ValidatorVerifier::from(&validator_set);
        let authors = multi_sig.get_signers_addresses(&verifier.get_ordered_account_addresses());
        verifier
            .check_voting_power(authors.iter(), true)
            .map_err(|_| Expected(ExpectedFailure::NotEnoughVotingPower))?;
        verifier
            .verify_multi_signatures(&observed, &multi_sig)
            .map_err(|_| Expected(ExpectedFailure::MultiSigVerificationFailed))?;

        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            observed.as_move_value(),
        ];
        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &TRUST_OBSERVATION_MODULE,
                UPSERT_OBSERVED_VALUE,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, UPSERT_OBSERVED_VALUE.as_str(), log_context)
            })
            .map_err(|result| Unexpected(result.unwrap_err()))?;

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
