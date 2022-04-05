// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! This crate defines [`trait StateView`](StateView).

use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use std::ops::Deref;

/// `StateView` is a trait that defines a read-only snapshot of the global state. It is passed to
/// the VM for transaction execution, during which the VM is guaranteed to read anything at the
/// given state.
pub trait StateView: Sync {
    /// For logging and debugging purpose, identifies what this view is for.
    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    /// Gets the state value for a given state key.
    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>>;

    /// VM needs this method to know whether the current state view is for genesis state creation.
    /// Currently TransactionPayload::WriteSet is only valid for genesis state creation.
    fn is_genesis(&self) -> bool;
}

#[derive(Copy, Clone)]
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

impl<R, S> StateView for R
where
    R: Deref<Target = S> + Sync,
    S: StateView,
{
    fn id(&self) -> StateViewId {
        self.deref().id()
    }

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<Vec<u8>>> {
        self.deref().get_state_value(state_key)
    }

    fn is_genesis(&self) -> bool {
        self.deref().is_genesis()
    }
}
