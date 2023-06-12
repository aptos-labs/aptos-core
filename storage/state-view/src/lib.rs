// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This crate defines [`trait StateView`](StateView).

use crate::{
    account_with_state_view::{AccountWithStateView, AsAccountWithStateView},
    in_memory_state_view::InMemoryStateView,
};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};
use std::ops::Deref;

pub mod account_with_state_cache;
pub mod account_with_state_view;
pub mod in_memory_state_view;

/// `StateView` is a trait that defines a read-only snapshot of the global state. It is passed to
/// the VM for transaction execution, during which the VM is guaranteed to read anything at the
/// given state.
pub trait TStateView {
    type Key;

    /// For logging and debugging purpose, identifies what this view is for.
    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    /// Gets the state value bytes for a given state key.
    fn get_state_value_bytes(&self, state_key: &Self::Key) -> Result<Option<Vec<u8>>> {
        let val_opt = self.get_state_value(state_key)?;
        Ok(val_opt.map(|val| val.into_bytes()))
    }

    /// Gets the state value for a given state key.
    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>>;

    /// VM needs this method to know whether the current state view is for genesis state creation.
    /// Currently TransactionPayload::WriteSet is only valid for genesis state creation.
    fn is_genesis(&self) -> bool;

    /// Get state storage usage info at epoch ending.
    fn get_usage(&self) -> Result<StateStorageUsage>;

    fn as_in_memory_state_view(&self) -> InMemoryStateView {
        unreachable!("in-memory state view conversion not supported yet")
    }
}

pub trait StateView: TStateView<Key = StateKey> {}

impl<T: TStateView<Key = StateKey>> StateView for T {}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StateViewId {
    /// State-sync applying a chunk of transactions.
    ChunkExecution { first_version: Version },
    /// LEC applying a block.
    BlockExecution { block_id: HashValue },
    /// VmValidator verifying incoming transaction.
    TransactionValidation { base_version: Version },
    /// For test, db-bootstrapper, etc. Usually not aimed to pass to VM.
    Miscellaneous,
}

impl<R, S, K> TStateView for R
where
    R: Deref<Target = S>,
    S: TStateView<Key = K>,
{
    type Key = K;

    fn id(&self) -> StateViewId {
        self.deref().id()
    }

    fn get_state_value(&self, state_key: &K) -> Result<Option<StateValue>> {
        self.deref().get_state_value(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.deref().is_genesis()
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.deref().get_usage()
    }
}

impl<'a, S: 'a + StateView> AsAccountWithStateView<'a> for S {
    fn as_account_with_state_view(
        &'a self,
        account_address: &'a AccountAddress,
    ) -> AccountWithStateView<'a> {
        AccountWithStateView::new(account_address, self)
    }
}
