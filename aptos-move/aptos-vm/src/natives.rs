// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use aptos_aggregator::resolver::TAggregatorV1View;
#[cfg(feature = "testing")]
use aptos_aggregator::{bounded_math::SignedU128, types::DelayedFieldsSpeculativeError};
#[cfg(feature = "testing")]
use aptos_aggregator::{resolver::TDelayedFieldView, types::DelayedFieldValue};
#[cfg(feature = "testing")]
use aptos_framework::natives::randomness::RandomnessContext;
#[cfg(feature = "testing")]
use aptos_framework::natives::{cryptography::algebra::AlgebraContext, event::NativeEventContext};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_native_interface::SafeNativeBuilder;
#[cfg(feature = "testing")]
use aptos_table_natives::{TableHandle, TableResolver};
use aptos_types::on_chain_config::{Features, TimedFeatures, TimedFeaturesBuilder};
#[cfg(feature = "testing")]
use aptos_types::{
    chain_id::ChainId,
    error::{PanicError, PanicOr},
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueMetadata},
    },
};
use aptos_vm_environment::natives::aptos_natives_with_builder;
#[cfg(feature = "testing")]
use bytes::Bytes;
#[cfg(feature = "testing")]
use move_binary_format::errors::PartialVMResult;
#[cfg(feature = "testing")]
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout};
use move_vm_runtime::native_functions::NativeFunctionTable;
#[cfg(feature = "testing")]
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
#[cfg(feature = "testing")]
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};
#[cfg(feature = "testing")]
use {
    aptos_framework::natives::{
        aggregator_natives::NativeAggregatorContext, code::NativeCodeContext,
        cryptography::ristretto255_point::NativeRistrettoPointContext,
        transaction_context::NativeTransactionContext,
    },
    move_vm_runtime::native_extensions::NativeContextExtensions,
    once_cell::sync::Lazy,
};

#[cfg(feature = "testing")]
struct AptosBlankStorage;

#[cfg(feature = "testing")]
impl AptosBlankStorage {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(feature = "testing")]
impl TAggregatorV1View for AptosBlankStorage {
    type Identifier = StateKey;

    fn get_aggregator_v1_state_value(
        &self,
        _id: &Self::Identifier,
    ) -> PartialVMResult<Option<StateValue>> {
        Ok(None)
    }
}

#[cfg(feature = "testing")]
impl TDelayedFieldView for AptosBlankStorage {
    type Identifier = DelayedFieldID;
    type ResourceGroupTag = StructTag;
    type ResourceKey = StateKey;

    fn get_delayed_field_value(
        &self,
        _id: &Self::Identifier,
    ) -> Result<DelayedFieldValue, PanicOr<DelayedFieldsSpeculativeError>> {
        unreachable!()
    }

    fn delayed_field_try_add_delta_outcome(
        &self,
        _id: &Self::Identifier,
        _base_delta: &SignedU128,
        _delta: &SignedU128,
        _max_value: u128,
    ) -> Result<bool, PanicOr<DelayedFieldsSpeculativeError>> {
        unreachable!()
    }

    fn generate_delayed_field_id(&self, _width: u32) -> Self::Identifier {
        unreachable!()
    }

    fn validate_delayed_field_id(&self, _id: &Self::Identifier) -> Result<(), PanicError> {
        unreachable!()
    }

    fn get_reads_needing_exchange(
        &self,
        _delayed_write_set_keys: &HashSet<Self::Identifier>,
        _skip: &HashSet<Self::ResourceKey>,
    ) -> Result<
        BTreeMap<Self::ResourceKey, (StateValueMetadata, u64, Arc<MoveTypeLayout>)>,
        PanicError,
    > {
        unreachable!()
    }

    fn get_group_reads_needing_exchange(
        &self,
        _delayed_write_set_keys: &HashSet<Self::Identifier>,
        _skip: &HashSet<Self::ResourceKey>,
    ) -> PartialVMResult<BTreeMap<Self::ResourceKey, (StateValueMetadata, u64)>> {
        unimplemented!()
    }
}

#[cfg(feature = "testing")]
impl TableResolver for AptosBlankStorage {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        _handle: &TableHandle,
        _key: &[u8],
        _layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<Option<Bytes>> {
        Ok(None)
    }
}

#[cfg(feature = "testing")]
#[allow(clippy::redundant_closure)]
static DUMMY_RESOLVER: Lazy<AptosBlankStorage> = Lazy::new(|| AptosBlankStorage::new());

pub fn aptos_natives(
    gas_feature_version: u64,
    native_gas_params: NativeGasParameters,
    misc_gas_params: MiscGasParameters,
    timed_features: TimedFeatures,
    features: Features,
) -> NativeFunctionTable {
    let mut builder = SafeNativeBuilder::new(
        gas_feature_version,
        native_gas_params,
        misc_gas_params,
        timed_features,
        features,
        None,
    );

    aptos_natives_with_builder(&mut builder, false)
}

pub fn assert_no_test_natives(err_msg: &str) {
    assert!(
        aptos_natives(
            LATEST_GAS_FEATURE_VERSION,
            NativeGasParameters::zeros(),
            MiscGasParameters::zeros(),
            TimedFeaturesBuilder::enable_all().build(),
            Features::default()
        )
        .into_iter()
        .all(|(_, module_name, func_name, _)| {
            !(module_name.as_str() == "unit_test"
                && func_name.as_str() == "create_signers_for_testing"
                || module_name.as_str() == "ed25519"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "ed25519" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "multi_ed25519"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "multi_ed25519" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "bls12381"
                    && func_name.as_str() == "generate_keys_internal"
                || module_name.as_str() == "bls12381" && func_name.as_str() == "sign_internal"
                || module_name.as_str() == "bls12381"
                    && func_name.as_str() == "generate_proof_of_possession_internal"
                || module_name.as_str() == "event"
                    && func_name.as_str() == "emitted_events_internal")
        }),
        "{}",
        err_msg
    )
}

#[cfg(feature = "testing")]
pub fn configure_for_unit_test() {
    move_unit_test::extensions::set_extension_hook(Box::new(unit_test_extensions_hook))
}

#[cfg(feature = "testing")]
fn unit_test_extensions_hook(exts: &mut NativeContextExtensions) {
    use aptos_framework::natives::object::NativeObjectContext;
    use aptos_table_natives::NativeTableContext;

    exts.add(NativeTableContext::new([0u8; 32], &*DUMMY_RESOLVER));
    exts.add(NativeCodeContext::new());
    exts.add(NativeTransactionContext::new(
        vec![1],
        vec![1],
        ChainId::test().id(),
        None,
        0,
    ));
    exts.add(NativeAggregatorContext::new(
        [0; 32],
        &*DUMMY_RESOLVER,
        false,
        &*DUMMY_RESOLVER,
    ));
    exts.add(NativeRistrettoPointContext::new());
    exts.add(AlgebraContext::new());
    exts.add(NativeEventContext::default());
    exts.add(NativeObjectContext::default());

    let mut randomness_ctx = RandomnessContext::new();
    randomness_ctx.mark_unbiasable();
    exts.add(randomness_ctx);
}
