// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{MoveResolverExt, SessionExt, SessionId},
    natives::aptos_natives,
};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext,
    code::NativeCodeContext,
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::errors::VMResult;
use move_bytecode_verifier::VerifierConfig;
use move_table_extension::NativeTableContext;
use move_vm_runtime::{
    config::VMConfig, move_vm::MoveVM, native_extensions::NativeContextExtensions,
};
use std::{ops::Deref, sync::Arc};

pub struct MoveVmExt {
    inner: MoveVM,
    chain_id: u8,
    features: Arc<Features>,
}

pub fn get_max_binary_format_version(features: &Features, gas_feature_version: u64) -> u32 {
    if features.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6) && gas_feature_version >= 5 {
        6
    } else {
        5
    }
}

impl MoveVmExt {
    pub fn new(
        native_gas_params: NativeGasParameters,
        abs_val_size_gas_params: AbstractValueSizeGasParameters,
        gas_feature_version: u64,
        chain_id: u8,
        features: Features,
        timed_features: TimedFeatures,
    ) -> VMResult<Self> {
        // Note: binary format v6 adds a few new integer types and their corresponding instructions.
        //       Therefore it depends on a new version of the gas schedule and cannot be allowed if
        //       the gas schedule hasn't been updated yet.
        let max_binary_format_version =
            get_max_binary_format_version(&features, gas_feature_version);

        let enable_invariant_violation_check_in_swap_loc =
            !timed_features.is_enabled(TimedFeatureFlag::DisableInvariantViolationCheckInSwapLoc);
        let type_size_limit = true;

        let verifier_config = verifier_config(&features, &timed_features);
        let features = Arc::new(features);

        Ok(Self {
            inner: MoveVM::new_with_config(
                aptos_natives(
                    native_gas_params,
                    abs_val_size_gas_params,
                    gas_feature_version,
                    timed_features,
                    features.clone(),
                ),
                VMConfig {
                    verifier: verifier_config,
                    max_binary_format_version,
                    paranoid_type_checks: crate::AptosVM::get_paranoid_checks(),
                    enable_invariant_violation_check_in_swap_loc,
                    type_size_limit,
                    max_value_nest_depth: Some(128),
                },
            )?,
            chain_id,
            features,
        })
    }

    pub fn new_session<'r, S: MoveResolverExt>(
        &self,
        remote: &'r S,
        session_id: SessionId,
        aggregator_enabled: bool,
    ) -> SessionExt<'r, '_> {
        let mut extensions = NativeContextExtensions::default();
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");

        extensions.add(NativeTableContext::new(txn_hash, remote));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(AlgebraContext::new());
        extensions.add(NativeAggregatorContext::new(
            txn_hash,
            remote,
            aggregator_enabled,
        ));

        let sender_opt = session_id.sender();
        let script_hash = match session_id {
            SessionId::Txn {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | SessionId::Prologue {
                sender: _,
                sequence_number: _,
                script_hash,
            }
            | SessionId::Epilogue {
                sender: _,
                sequence_number: _,
                script_hash,
            } => script_hash,
            _ => vec![],
        };

        extensions.add(NativeTransactionContext::new(
            txn_hash.to_vec(),
            script_hash,
            self.chain_id,
        ));
        extensions.add(NativeCodeContext::default());
        extensions.add(NativeStateStorageContext::new(remote));

        // The VM code loader has bugs around module upgrade. After a module upgrade, the internal
        // cache needs to be flushed to work around those bugs.
        self.inner.flush_loader_cache_if_invalidated();

        SessionExt::new(
            self.inner.new_session_with_extensions(remote, extensions),
            remote,
            sender_opt,
            self.features.clone(),
        )
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub fn verifier_config(features: &Features, _timed_features: &TimedFeatures) -> VerifierConfig {
    VerifierConfig {
        max_loop_depth: Some(5),
        max_generic_instantiation_length: Some(32),
        max_function_parameters: Some(128),
        max_basic_blocks: Some(1024),
        max_value_stack_size: 1024,
        max_type_nodes: Some(256),
        max_dependency_depth: Some(256),
        max_push_size: Some(10000),
        max_struct_definitions: None,
        max_fields_in_struct: None,
        max_function_definitions: None,
        max_back_edges_per_function: None,
        max_back_edges_per_module: None,
        max_basic_blocks_in_script: None,
        max_per_fun_meter_units: Some(1000 * 80000),
        max_per_mod_meter_units: Some(1000 * 80000),
        use_signature_checker_v2: features.is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2),
    }
}
