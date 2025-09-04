// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    velor_vm::get_system_transaction_output,
    errors::expect_only_successful_execution,
    move_vm_ext::{VelorMoveResolver, SessionId},
    system_module_names::{JWKS_MODULE, UPSERT_INTO_OBSERVED_JWKS},
    validator_txns::jwk::{
        ExecutionFailure::{Expected, Unexpected},
        ExpectedFailure::{
            IncorrectVersion, MissingResourceObservedJWKs, MissingResourceValidatorSet,
            MultiSigVerificationFailed, NotEnoughVotingPower,
        },
    },
    VelorVM,
};
use velor_logger::debug;
use velor_types::{
    jwks,
    jwks::{Issuer, ObservedJWKs, ProviderJWKs, QuorumCertifiedUpdate},
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::{OnChainConfig, ValidatorSet},
    transaction::TransactionStatus,
    validator_verifier::ValidatorVerifier,
};
use velor_vm_logging::log_schema::AdapterLogSchema;
use velor_vm_types::{
    module_and_script_storage::module_storage::VelorModuleStorage, output::VMOutput,
};
use move_core_types::{
    account_address::AccountAddress,
    value::{serialize_values, MoveValue},
    vm_status::{AbortLocation, StatusCode, VMStatus},
};
use move_vm_runtime::module_traversal::{TraversalContext, TraversalStorage};
use move_vm_types::gas::UnmeteredGasMeter;
use std::collections::HashMap;

#[derive(Debug)]
enum ExpectedFailure {
    // Move equivalent: `errors::invalid_argument(*)`
    IncorrectVersion = 0x010103,
    MultiSigVerificationFailed = 0x010104,
    NotEnoughVotingPower = 0x010105,

    // Move equivalent: `errors::invalid_state(*)`
    MissingResourceValidatorSet = 0x30101,
    MissingResourceObservedJWKs = 0x30102,
}

enum ExecutionFailure {
    Expected(ExpectedFailure),
    Unexpected(VMStatus),
}

impl VelorVM {
    pub(crate) fn process_jwk_update(
        &self,
        resolver: &impl VelorMoveResolver,
        module_storage: &impl VelorModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: jwks::QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        debug!("Processing jwk transaction");
        match self.process_jwk_update_inner(
            resolver,
            module_storage,
            log_context,
            session_id,
            update,
        ) {
            Ok((vm_status, vm_output)) => {
                debug!("Processing jwk transaction ok.");
                Ok((vm_status, vm_output))
            },
            Err(Expected(failure)) => {
                // Pretend we are inside Move, and expected failures are like Move aborts.
                debug!("Processing dkg transaction expected failure: {:?}", failure);
                Ok((
                    VMStatus::MoveAbort(AbortLocation::Script, failure as u64),
                    VMOutput::empty_with_status(TransactionStatus::Discard(StatusCode::ABORTED)),
                ))
            },
            Err(Unexpected(vm_status)) => {
                debug!(
                    "Processing jwk transaction unexpected failure: {:?}",
                    vm_status
                );
                Err(vm_status)
            },
        }
    }

    fn process_jwk_update_inner(
        &self,
        resolver: &impl VelorMoveResolver,
        module_storage: &impl VelorModuleStorage,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: jwks::QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        // Load resources.
        let validator_set =
            ValidatorSet::fetch_config(resolver).ok_or(Expected(MissingResourceValidatorSet))?;
        let observed_jwks =
            ObservedJWKs::fetch_config(resolver).ok_or(Expected(MissingResourceObservedJWKs))?;

        let mut jwks_by_issuer: HashMap<Issuer, ProviderJWKs> =
            observed_jwks.into_providers_jwks().into();
        let issuer = update.update.issuer.clone();
        let on_chain = jwks_by_issuer
            .entry(issuer.clone())
            .or_insert_with(|| ProviderJWKs::new(issuer));
        let verifier = ValidatorVerifier::from(&validator_set);

        let QuorumCertifiedUpdate {
            update: observed,
            multi_sig,
        } = update;

        // Check version.
        if on_chain.version + 1 != observed.version {
            return Err(Expected(IncorrectVersion));
        }

        let authors = multi_sig.get_signers_addresses(&verifier.get_ordered_account_addresses());

        // Check voting power.
        verifier
            .check_voting_power(authors.iter(), true)
            .map_err(|_| Expected(NotEnoughVotingPower))?;

        // Verify multi-sig.
        verifier
            .verify_multi_signatures(&observed, &multi_sig)
            .map_err(|_| Expected(MultiSigVerificationFailed))?;

        // All verification passed. Apply the `observed`.
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id, None);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            vec![observed].as_move_value(),
        ];

        let traversal_storage = TraversalStorage::new();
        session
            .execute_function_bypass_visibility(
                &JWKS_MODULE,
                UPSERT_INTO_OBSERVED_JWKS,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
                &mut TraversalContext::new(&traversal_storage),
                module_storage,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, UPSERT_INTO_OBSERVED_JWKS.as_str(), log_context)
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
