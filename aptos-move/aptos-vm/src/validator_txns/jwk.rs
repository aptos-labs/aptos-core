// Copyright © Aptos Foundation

use std::collections::HashMap;
use aptos_bitvec::BitVec;
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::fee_statement::FeeStatement;
use aptos_types::jwks;
use aptos_types::jwks::{Issuer, ObservedJWKs, ProviderJWKs, QuorumCertifiedUpdate};
use aptos_types::move_utils::as_move_value::AsMoveValue;
use aptos_types::on_chain_config::{OnChainConfig, ValidatorSet};
use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_vm_logging::log_schema::AdapterLogSchema;
use aptos_vm_types::output::VMOutput;
use move_core_types::account_address::AccountAddress;
use move_core_types::value::{MoveValue, serialize_values};
use move_core_types::vm_status::{AbortLocation, StatusCode, VMStatus};
use move_vm_types::gas::UnmeteredGasMeter;
use crate::aptos_vm::get_or_vm_startup_failure;
use crate::AptosVM;
use crate::errors::expect_only_successful_execution;
use crate::move_vm_ext::{AptosMoveResolver, SessionId};
use crate::system_module_names::{JWKS_MODULE, UPSERT_INTO_OBSERVED_JWKS};
use crate::validator_txns::jwk::ExecutionFailure::{Expected, Unexpected};
use crate::validator_txns::jwk::ExpectedFailure::{IncorrectVersion, MissingResourceObservedJWKs, MissingResourceValidatorSet, MultiSigVerificationFailed, NotEnoughVotingPower};

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

impl AptosVM {
    pub(crate) fn process_jwk_update(
        &self,
        resolver: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: jwks::QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), VMStatus> {
        match self.process_jwk_update_inner(resolver, log_context, session_id, update) {
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

    fn process_jwk_update_inner(
        &self,
        resolver: &impl AptosMoveResolver,
        log_context: &AdapterLogSchema,
        session_id: SessionId,
        update: jwks::QuorumCertifiedUpdate,
    ) -> Result<(VMStatus, VMOutput), ExecutionFailure> {
        // Load resources.
        let validator_set = ValidatorSet::fetch_config(resolver).ok_or_else(||Expected(MissingResourceValidatorSet))?;
        let observed_jwks = ObservedJWKs::fetch_config(resolver).ok_or_else(||Expected(MissingResourceObservedJWKs))?;

        let mut jwks_by_issuer: HashMap<Issuer, ProviderJWKs> = observed_jwks.into_providers_jwks().into();
        let issuer = update.update.issuer.clone();
        let on_chain = jwks_by_issuer
            .entry(issuer.clone())
            .or_insert_with(|| ProviderJWKs::new(issuer));
        let verifier = ValidatorVerifier::from(&validator_set);

        let QuorumCertifiedUpdate {
            authors,
            update: observed,
            multi_sig,
        } = update;

        // Check version.
        if on_chain.version + 1 != observed.version {
            return Err(Expected(IncorrectVersion));
        }

        let signer_bit_vec = BitVec::from(
            verifier
                .get_ordered_account_addresses()
                .into_iter()
                .map(|addr| authors.contains(&addr))
                .collect::<Vec<_>>(),
        );

        // Verify multi-sig.
        verifier.verify_multi_signatures(
            &observed,
            &AggregateSignature::new(signer_bit_vec, Some(multi_sig)),
        ).map_err(|_|Expected(MultiSigVerificationFailed))?;

        // Check voting power.
        verifier.check_voting_power(authors.iter(), true).map_err(|_|Expected(NotEnoughVotingPower))?;

        // All verification passed. Apply the `observed`.
        let mut gas_meter = UnmeteredGasMeter;
        let mut session = self.new_session(resolver, session_id);
        let args = vec![
            MoveValue::Signer(AccountAddress::ONE),
            vec![observed].as_move_value(),
        ];

        session
            .execute_function_bypass_visibility(
                &JWKS_MODULE,
                UPSERT_INTO_OBSERVED_JWKS,
                vec![],
                serialize_values(&args),
                &mut gas_meter,
            )
            .map_err(|e| {
                expect_only_successful_execution(e, UPSERT_INTO_OBSERVED_JWKS.as_str(), log_context)
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
