// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    move_vm_ext::{warm_vm_cache::WarmVmCache, AptosMoveResolver, SessionExt, SessionId},
    natives::aptos_natives_with_builder,
};
use aptos_crypto::HashValue;
use aptos_gas_algebra::DynamicExpression;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_native_interface::SafeNativeBuilder;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{FeatureFlag, Features, TimedFeaturesBuilder},
    transaction::user_transaction_context::UserTransactionContext,
    vm::configs::aptos_prod_vm_config,
};
use aptos_vm_types::{environment::Environment, storage::change_set_configs::ChangeSetConfigs};
use move_vm_runtime::move_vm::MoveVM;
use std::{ops::Deref, sync::Arc};

/// MoveVM wrapper which is used to run genesis initializations. Designed as a
/// stand-alone struct to ensure all genesis configurations are in one place,
/// and are modified accordingly. The VM is initialized with default parameters,
/// and should only be used to run genesis sessions.
pub struct GenesisMoveVM {
    vm: MoveVM,
    features: Features,
}

impl GenesisMoveVM {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let features = Features::default();
        let timed_features = TimedFeaturesBuilder::enable_all().build();

        // Genesis runs sessions, where there is no concept of block execution.
        // Hence, delayed fields are not enabled.
        let delayed_field_optimization_enabled = false;
        let vm_config = aptos_prod_vm_config(
            &features,
            &timed_features,
            delayed_field_optimization_enabled,
        );

        // All genesis sessions run with unmetered gas meter, and here we set
        // the gas parameters for natives as zeros (because they do not matter).
        let mut native_builder = SafeNativeBuilder::new(
            LATEST_GAS_FEATURE_VERSION,
            NativeGasParameters::zeros(),
            MiscGasParameters::zeros(),
            timed_features.clone(),
            features.clone(),
        );

        let vm = MoveVM::new_with_config(
            aptos_natives_with_builder(&mut native_builder),
            vm_config.clone(),
        );

        Self { vm, features }
    }

    pub fn genesis_change_set_configs(&self) -> ChangeSetConfigs {
        // Because genesis sessions are not metered, there are no change set
        // (storage) costs as well.
        ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION)
    }

    pub fn new_genesis_session<'r, R: AptosMoveResolver>(
        &self,
        resolver: &'r R,
        genesis_id: HashValue,
    ) -> SessionExt<'r, '_> {
        let chain_id = ChainId::test();
        let session_id = SessionId::genesis(genesis_id);
        SessionExt::new(
            session_id,
            &self.vm,
            chain_id,
            &self.features,
            None,
            resolver,
        )
    }
}

pub struct MoveVmExt {
    inner: MoveVM,
    pub(crate) env: Arc<Environment>,
}

impl MoveVmExt {
    fn new_impl<F>(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        env: Arc<Environment>,
        gas_hook: Option<F>,
        resolver: &impl AptosMoveResolver,
    ) -> Self
    where
        F: Fn(DynamicExpression) + Send + Sync + 'static,
    {
        let mut builder = SafeNativeBuilder::new(
            gas_feature_version,
            native_gas_params.clone(),
            misc_gas_params.clone(),
            env.timed_features.clone(),
            env.features.clone(),
        );
        if let Some(hook) = gas_hook {
            builder.set_gas_hook(hook);
        }

        Self {
            inner: WarmVmCache::get_warm_vm(
                builder,
                &env.vm_config,
                resolver,
                env.features.is_enabled(FeatureFlag::VM_BINARY_FORMAT_V7),
            )
            .expect("should be able to create Move VM; check if there are duplicated natives"),
            env,
        }
    }

    pub fn new(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        env: Arc<Environment>,
        resolver: &impl AptosMoveResolver,
    ) -> Self {
        Self::new_impl::<fn(DynamicExpression)>(
            native_gas_params,
            misc_gas_params,
            gas_feature_version,
            env,
            None,
            resolver,
        )
    }

    pub fn new_with_gas_hook<F>(
        native_gas_params: NativeGasParameters,
        misc_gas_params: MiscGasParameters,
        gas_feature_version: u64,
        env: Arc<Environment>,
        gas_hook: Option<F>,
        resolver: &impl AptosMoveResolver,
    ) -> Self
    where
        F: Fn(DynamicExpression) + Send + Sync + 'static,
    {
        Self::new_impl(
            native_gas_params,
            misc_gas_params,
            gas_feature_version,
            env,
            gas_hook,
            resolver,
        )
    }

    pub fn new_session<'r, R: AptosMoveResolver>(
        &self,
        resolver: &'r R,
        session_id: SessionId,
        maybe_user_transaction_context: Option<UserTransactionContext>,
    ) -> SessionExt<'r, '_> {
        SessionExt::new(
            session_id,
            &self.inner,
            self.env.chain_id,
            &self.env.features,
            maybe_user_transaction_context,
            resolver,
        )
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
