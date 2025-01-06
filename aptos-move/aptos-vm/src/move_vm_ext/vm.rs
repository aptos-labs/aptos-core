// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{warm_vm_cache::WarmVmCache, AptosMoveResolver, SessionExt, SessionId};
use aptos_crypto::HashValue;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_native_interface::SafeNativeBuilder;
use aptos_types::{
    chain_id::ChainId,
    on_chain_config::{Features, TimedFeaturesBuilder},
    transaction::user_transaction_context::UserTransactionContext,
};
use aptos_vm_environment::{
    environment::AptosEnvironment,
    natives::aptos_natives_with_builder,
    prod_configs::{aptos_default_ty_builder, aptos_prod_vm_config},
};
use aptos_vm_types::storage::change_set_configs::ChangeSetConfigs;
use move_vm_runtime::{move_vm::MoveVM, RuntimeEnvironment, WithRuntimeEnvironment};
use std::ops::Deref;

/// Used by genesis to create runtime environment and VM ([GenesisMoveVM]), encapsulating all
/// configs.
pub struct GenesisRuntimeBuilder {
    chain_id: ChainId,
    features: Features,
    runtime_environment: RuntimeEnvironment,
}

impl GenesisRuntimeBuilder {
    /// Returns a builder, capable of creating VM and runtime environment to run genesis.
    pub fn new(chain_id: ChainId) -> Self {
        let features = Features::default();
        let timed_features = TimedFeaturesBuilder::enable_all().build();

        let vm_config =
            aptos_prod_vm_config(&features, &timed_features, aptos_default_ty_builder());

        // All genesis sessions run with unmetered gas meter, and here we set
        // the gas parameters for natives as zeros (because they do not matter).
        let mut native_builder = SafeNativeBuilder::new(
            LATEST_GAS_FEATURE_VERSION,
            NativeGasParameters::zeros(),
            MiscGasParameters::zeros(),
            timed_features.clone(),
            features.clone(),
            None,
        );
        let natives = aptos_natives_with_builder(&mut native_builder, false);
        let runtime_environment = RuntimeEnvironment::new_with_config(natives, vm_config);
        Self {
            chain_id,
            features,
            runtime_environment,
        }
    }

    /// Returns the runtime environment used for any genesis sessions.
    pub fn build_genesis_runtime_environment(&self) -> RuntimeEnvironment {
        self.runtime_environment.clone()
    }

    /// Returns MoveVM for the genesis.
    pub fn build_genesis_vm(&self) -> GenesisMoveVM {
        GenesisMoveVM {
            vm: MoveVM::new_with_runtime_environment(&self.runtime_environment),
            chain_id: self.chain_id,
            features: self.features.clone(),
        }
    }
}

/// MoveVM wrapper which is used to run genesis initializations. Designed as a stand-alone struct
/// to ensure all genesis configurations are in one place, and are modified accordingly. The VM is
/// created via [GenesisRuntimeBuilder], and should only be used to run genesis sessions.
pub struct GenesisMoveVM {
    vm: MoveVM,
    chain_id: ChainId,
    features: Features,
}

impl GenesisMoveVM {
    /// Returns a new genesis session.
    pub fn new_genesis_session<'r, R: AptosMoveResolver>(
        &self,
        resolver: &'r R,
        genesis_id: HashValue,
    ) -> SessionExt<'r, '_> {
        let session_id = SessionId::genesis(genesis_id);
        SessionExt::new(
            session_id,
            &self.vm,
            self.chain_id,
            &self.features,
            None,
            resolver,
        )
    }

    /// Returns the set of features used by genesis VM.
    pub fn genesis_features(&self) -> &Features {
        &self.features
    }

    /// Returns change set configs used by genesis VM sessions. Because genesis sessions are not
    /// metered, there are no change set (storage) costs.
    pub fn genesis_change_set_configs(&self) -> ChangeSetConfigs {
        ChangeSetConfigs::unlimited_at_gas_feature_version(LATEST_GAS_FEATURE_VERSION)
    }
}

pub struct MoveVmExt {
    inner: MoveVM,
    pub(crate) env: AptosEnvironment,
}

impl MoveVmExt {
    pub fn new(env: AptosEnvironment, resolver: &impl AptosMoveResolver) -> Self {
        let vm = if env.features().is_loader_v2_enabled() {
            MoveVM::new_with_runtime_environment(env.runtime_environment())
        } else {
            WarmVmCache::get_warm_vm(&env, resolver)
                .expect("should be able to create Move VM; check if there are duplicated natives")
        };

        Self { inner: vm, env }
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
            self.env.chain_id(),
            self.env.features(),
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
