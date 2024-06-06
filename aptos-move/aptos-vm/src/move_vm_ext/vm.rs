// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_vm_ext::{warm_vm_cache::WarmVmCache, AptosMoveResolver, SessionExt, SessionId};
use aptos_framework::natives::{
    aggregator_natives::NativeAggregatorContext,
    code::NativeCodeContext,
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    object::NativeObjectContext,
    randomness::RandomnessContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_gas_algebra::DynamicExpression;
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_native_interface::SafeNativeBuilder;
use aptos_table_natives::NativeTableContext;
use aptos_types::{
    on_chain_config::FeatureFlag, transaction::user_transaction_context::UserTransactionContext,
    vm::environment::Environment,
};
use move_vm_runtime::{move_vm::MoveVM, native_extensions::NativeContextExtensions};
use std::{ops::Deref, sync::Arc};

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

    pub fn new_session<'r, S: AptosMoveResolver>(
        &self,
        resolver: &'r S,
        session_id: SessionId,
        user_transaction_context_opt: Option<UserTransactionContext>,
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
        extensions.add(NativeAggregatorContext::new(
            txn_hash,
            resolver,
            self.env.vm_config.delayed_field_optimization_enabled,
            resolver,
        ));
        extensions.add(RandomnessContext::new());
        extensions.add(NativeTransactionContext::new(
            txn_hash.to_vec(),
            session_id.into_script_hash(),
            self.env.chain_id.id(),
            user_transaction_context_opt,
        ));
        extensions.add(NativeCodeContext::default());
        extensions.add(NativeStateStorageContext::new(resolver));
        extensions.add(NativeEventContext::default());
        extensions.add(NativeObjectContext::default());

        // The VM code loader has bugs around module upgrade. After a module upgrade, the internal
        // cache needs to be flushed to work around those bugs.
        self.inner.flush_loader_cache_if_invalidated();

        SessionExt::new(
            self.inner.new_session_with_extensions(resolver, extensions),
            resolver,
            self.env.features.is_storage_slot_metadata_enabled(),
        )
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
