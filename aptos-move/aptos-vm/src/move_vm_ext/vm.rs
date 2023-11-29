// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{warm_vm_cache::WarmVmCache, AptosMoveResolver, SessionExt, SessionId};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext,
    code::NativeCodeContext,
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_gas_algebra::DynamicExpression;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_native_interface::SafeNativeBuilder;
use aptos_table_natives::NativeTableContext;
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatureFlag, TimedFeatures};
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::VMResult,
    file_format_common,
    file_format_common::{IDENTIFIER_SIZE_MAX, LEGACY_IDENTIFIER_SIZE_MAX},
};
use move_bytecode_verifier::VerifierConfig;
use move_vm_runtime::{
    config::VMConfig, move_vm::MoveVM, native_extensions::NativeContextExtensions,
};
use std::{ops::Deref, sync::Arc};

pub struct MoveVmExt {
    inner: MoveVM,
    chain_id: u8,
    features: Arc<Features>,
}

pub fn get_max_binary_format_version(
    features: &Features,
    gas_feature_version_opt: Option<u64>,
) -> u32 {
    // For historical reasons, we support still < gas version 5, but if a new caller don't specify
    // the gas version, we default to 5, which was introduced in late '22.
    let gas_feature_version = gas_feature_version_opt.unwrap_or(5);
    if gas_feature_version < 5 {
        file_format_common::VERSION_5
    } else if features.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V7) {
        file_format_common::VERSION_7
    } else if features.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V6) {
        file_format_common::VERSION_6
    } else {
        file_format_common::VERSION_5
    }
}

pub fn get_max_identifier_size(features: &Features) -> u64 {
    if features.is_enabled(FeatureFlag::LIMIT_MAX_IDENTIFIER_LENGTH) {
        IDENTIFIER_SIZE_MAX
    } else {
        LEGACY_IDENTIFIER_SIZE_MAX
    }
}

impl MoveVmExt {
    fn new_impl<F>(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        chain_id: u8,
        features: Features,
        timed_features: TimedFeatures,
        gas_hook: Option<F>,
        resolver: &impl AptosMoveResolver,
    ) -> VMResult<Self>
    where
        F: Fn(DynamicExpression) + Send + Sync + 'static,
    {
        // Note: binary format v6 adds a few new integer types and their corresponding instructions.
        //       Therefore it depends on a new version of the gas schedule and cannot be allowed if
        //       the gas schedule hasn't been updated yet.
        let max_binary_format_version =
            get_max_binary_format_version(&features, Some(gas_feature_version));

        let max_identifier_size = get_max_identifier_size(&features);

        let enable_invariant_violation_check_in_swap_loc =
            !timed_features.is_enabled(TimedFeatureFlag::DisableInvariantViolationCheckInSwapLoc);
        let type_size_limit = true;

        let verifier_config = verifier_config(&features, &timed_features);

        let mut type_max_cost = 0;
        let mut type_base_cost = 0;
        let mut type_byte_cost = 0;
        if timed_features.is_enabled(TimedFeatureFlag::LimitTypeTagSize) {
            // 5000 limits type tag total size < 5000 bytes and < 50 nodes
            type_max_cost = 5000;
            type_base_cost = 100;
            type_byte_cost = 1;
        }

        // If aggregator execution is enabled, we need to tag aggregator_v2 types,
        // so they can be exchanged with identifiers during VM execution.
        let aggregator_v2_type_tagging = features.is_aggregator_v2_delayed_fields_enabled();

        let mut builder = SafeNativeBuilder::new(
            gas_feature_version,
            native_gas_params.clone(),
            misc_gas_params.clone(),
            timed_features.clone(),
            features.clone(),
        );

        if let Some(hook) = gas_hook {
            builder.set_gas_hook(hook);
        }

        Ok(Self {
            inner: WarmVmCache::get_warm_vm(
                builder,
                VMConfig {
                    verifier: verifier_config,
                    deserializer_config: DeserializerConfig::new(
                        max_binary_format_version,
                        max_identifier_size,
                    ),
                    paranoid_type_checks: crate::AptosVM::get_paranoid_checks(),
                    enable_invariant_violation_check_in_swap_loc,
                    type_size_limit,
                    max_value_nest_depth: Some(128),
                    type_max_cost,
                    type_base_cost,
                    type_byte_cost,
                    aggregator_v2_type_tagging,
                },
                resolver,
            )?,
            chain_id,
            features: Arc::new(features),
        })
    }

    pub fn new(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        chain_id: u8,
        features: Features,
        timed_features: TimedFeatures,
        resolver: &impl AptosMoveResolver,
    ) -> VMResult<Self> {
        Self::new_impl::<fn(DynamicExpression)>(
            native_gas_params,
            misc_gas_params,
            gas_feature_version,
            chain_id,
            features,
            timed_features,
            None,
            resolver,
        )
    }

    pub fn new_with_gas_hook<F>(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        chain_id: u8,
        features: Features,
        timed_features: TimedFeatures,
        gas_hook: Option<F>,
        resolver: &impl AptosMoveResolver,
    ) -> VMResult<Self>
    where
        F: Fn(DynamicExpression) + Send + Sync + 'static,
    {
        Self::new_impl(
            native_gas_params,
            misc_gas_params,
            gas_feature_version,
            chain_id,
            features,
            timed_features,
            gas_hook,
            resolver,
        )
    }

    pub fn new_session<'r, S: AptosMoveResolver>(
        &self,
        resolver: &'r S,
        session_id: SessionId,
    ) -> SessionExt<'r, '_> {
        let mut extensions = NativeContextExtensions::default();
        let txn_hash: [u8; 32] = session_id
            .as_uuid()
            .to_vec()
            .try_into()
            .expect("HashValue should convert to [u8; 32]");

        extensions.add(NativeTableContext::new(txn_hash, resolver));
        extensions.add(NativeRistrettoPointContext::new());
        extensions.add(AlgebraContext::new());
        extensions.add(NativeAggregatorContext::new(txn_hash, resolver, resolver));

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
        extensions.add(NativeStateStorageContext::new(resolver));
        extensions.add(NativeEventContext::default());

        // The VM code loader has bugs around module upgrade. After a module upgrade, the internal
        // cache needs to be flushed to work around those bugs.
        self.inner.flush_loader_cache_if_invalidated();

        SessionExt::new(
            self.inner.new_session_with_extensions(resolver, extensions),
            resolver,
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
        sig_checker_v2_fix_script_ty_param_count: features
            .is_enabled(FeatureFlag::SIGNATURE_CHECKER_V2_SCRIPT_FIX),
    }
}
