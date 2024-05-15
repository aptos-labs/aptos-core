// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    environment::aptos_prod_vm_config,
    move_vm_ext::{warm_vm_cache::WarmVmCache, AptosMoveResolver, SessionExt, SessionId},
};
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
    chain_id::ChainId,
    on_chain_config::{Features, TimedFeatures},
    transaction::user_transaction_context::UserTransactionContext,
};
use move_binary_format::errors::VMResult;
use move_vm_runtime::{move_vm::MoveVM, native_extensions::NativeContextExtensions};
use std::ops::Deref;

pub struct MoveVmExt {
    inner: MoveVM,
    chain_id: u8,
    features: Features,
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
        aggregator_v2_type_tagging: bool,
    ) -> VMResult<Self>
    where
        F: Fn(DynamicExpression) + Send + Sync + 'static,
    {
        let vm_config = aptos_prod_vm_config(
            &features,
            &timed_features,
            gas_feature_version,
            aggregator_v2_type_tagging,
        );
        let mut builder = SafeNativeBuilder::new(
            gas_feature_version,
            native_gas_params,
            misc_gas_params,
            timed_features,
            features.clone(),
        );

        if let Some(hook) = gas_hook {
            builder.set_gas_hook(hook);
        }

        Ok(Self {
            inner: WarmVmCache::get_warm_vm(builder, vm_config, resolver)?,
            chain_id,
            features,
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
        aggregator_v2_type_tagging: bool,
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
            aggregator_v2_type_tagging,
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
        aggregator_v2_type_tagging: bool,
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
            aggregator_v2_type_tagging,
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
        extensions.add(NativeAggregatorContext::new(txn_hash, resolver, resolver));
        extensions.add(RandomnessContext::new());
        extensions.add(NativeTransactionContext::new(
            txn_hash.to_vec(),
            session_id.into_script_hash(),
            self.chain_id,
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
            self.features.is_storage_slot_metadata_enabled(),
        )
    }

    pub(crate) fn features(&self) -> &Features {
        &self.features
    }

    pub fn chain_id(&self) -> ChainId {
        ChainId::new(self.chain_id)
    }
}

impl Deref for MoveVmExt {
    type Target = MoveVM;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
