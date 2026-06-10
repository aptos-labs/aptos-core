// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native context extensions for the legacy MoveVM, built the same way as
//! `aptos-vm`'s `make_aptos_extensions`, but over a self-contained state view
//! instead of an `AptosMoveResolver`.
//!
//! Each context is handed a [`HarnessView`], which implements `TStateView`
//! (and therefore, via blanket impls, the aggregator / delayed-field /
//! state-storage resolvers) plus `TableResolver`. That covers every resolver
//! the contexts require, with no dependency on the aptos-vm resolver stack.

use aptos_framework_natives::{
    aggregator_natives::NativeAggregatorContext,
    code::NativeCodeContext,
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    object::NativeObjectContext,
    randomness::RandomnessContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use crate::txn::EntryCall;
use aptos_table_natives::{NativeTableContext, TableHandle, TableResolver};
use aptos_types::{
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
        StateViewResult, TStateView,
    },
    transaction::{
        user_transaction_context::{EntryFunctionPayload, UserTransactionContext},
        AuxiliaryInfo, PersistedAuxiliaryInfo, ReplayProtector, SignedTransaction,
    },
};
use aptos_vm::move_vm_ext::SessionId;
use bytes::Bytes;
use move_binary_format::errors::PartialVMError;
use move_core_types::value::MoveTypeLayout;
use move_vm_runtime::native_extensions::NativeContextExtensions;
use std::collections::BTreeMap;

/// A state view over the captured raw state (resource-group blobs and table
/// items intact), used to back the native context extensions. Implementing
/// `TStateView` yields the aggregator / delayed-field / state-storage resolvers
/// via blanket impls; `TableResolver` is implemented directly.
pub struct HarnessView {
    state: BTreeMap<StateKey, StateValue>,
}

impl HarnessView {
    pub fn new(state: BTreeMap<StateKey, StateValue>) -> Self {
        Self { state }
    }
}

impl TStateView for HarnessView {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> StateViewResult<Option<StateValue>> {
        Ok(self.state.get(state_key).cloned())
    }

    fn get_usage(&self) -> StateViewResult<StateStorageUsage> {
        Ok(StateStorageUsage::new_untracked())
    }
}

impl TableResolver for HarnessView {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<Bytes>, PartialVMError> {
        let state_key = StateKey::table_item(&(*handle).into(), key);
        Ok(self.state.get(&state_key).map(|value| value.bytes().clone()))
    }
}

/// Builds the native context extensions for replaying `entry` over `view`.
/// Derives the session id from the transaction's replay protector and a
/// [`UserTransactionContext`] (so `0x1::transaction_context` natives, and the
/// monotonic transaction index some entrypoints branch on, match the on-chain
/// values), then assembles the contexts. Chain id is mainnet. Shared by the
/// CLI runner (`v1`) and the criterion benchmark so the setup is not
/// duplicated.
pub fn replay_extensions<'a>(
    view: &'a HarnessView,
    signed: &SignedTransaction,
    aux_info: PersistedAuxiliaryInfo,
    entry: &EntryCall,
) -> NativeContextExtensions<'a> {
    let session_id = match signed.replay_protector() {
        ReplayProtector::SequenceNumber(sequence_number) => SessionId::Txn {
            sender: entry.sender,
            sequence_number,
            script_hash: vec![],
        },
        ReplayProtector::Nonce(nonce) => SessionId::OrderlessTxn {
            sender: entry.sender,
            nonce,
            expiration_time: signed.expiration_timestamp_secs(),
            script_hash: vec![],
        },
    };
    let entry_payload = EntryFunctionPayload::new(
        *entry.module.address(),
        entry.module.name().to_string(),
        entry.function.to_string(),
        entry
            .ty_args
            .iter()
            .map(|t| t.to_canonical_string())
            .collect(),
        entry.args.to_vec(),
    );
    let user_txn_ctx = UserTransactionContext::new(
        entry.sender,
        vec![],
        entry.sender,
        signed.max_gas_amount(),
        signed.gas_unit_price(),
        1,
        Some(entry_payload),
        None,
        AuxiliaryInfo::new(aux_info, None).transaction_index_kind(),
        false,
    );
    make_extensions(view, &session_id, 1, Some(user_txn_ctx))
}

/// Builds the Aptos native context extensions over `view`, mirroring
/// `aptos-vm`'s `make_aptos_extensions`, with delayed-field optimization off.
pub fn make_extensions<'a>(
    view: &'a HarnessView,
    session_id: &SessionId,
    chain_id: u8,
    user_transaction_context: Option<UserTransactionContext>,
) -> NativeContextExtensions<'a> {
    let txn_hash = session_id.txn_hash();
    // `SessionId::session_counter`/`into_script_hash` are not public, so derive
    // the same values from the variant we constructed.
    let (script_hash, session_counter) = match session_id {
        SessionId::Txn { script_hash, .. } => (script_hash.clone(), 35u8),
        SessionId::OrderlessTxn { script_hash, .. } => (script_hash.clone(), 40u8),
        _ => (vec![], 0u8),
    };

    let mut extensions = NativeContextExtensions::default();
    extensions.add(NativeTableContext::new(txn_hash, view));
    extensions.add(NativeRistrettoPointContext::new());
    extensions.add(AlgebraContext::new());
    extensions.add(NativeAggregatorContext::new(txn_hash, view, false, view));
    extensions.add(RandomnessContext::new());
    extensions.add(NativeTransactionContext::new(
        txn_hash.to_vec(),
        script_hash,
        chain_id,
        user_transaction_context,
        session_counter,
    ));
    extensions.add(NativeCodeContext::new());
    extensions.add(NativeStateStorageContext::new(view));
    extensions.add(NativeEventContext::default());
    extensions.add(NativeObjectContext::default());
    extensions
}
