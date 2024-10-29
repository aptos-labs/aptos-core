// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::{state_authenticator::StateAuthenticator, state_delta::StateDelta};
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Default, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl StateCheckpointOutput {
    pub fn new(
        parent_auth: StateAuthenticator,
        last_checkpoint_auth: Option<StateAuthenticator>,
        state_auth: StateAuthenticator,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
    ) -> Self {
        Self::new_impl(Inner {
            parent_auth,
            last_checkpoint_auth,
            state_auth,
            state_checkpoint_hashes,
        })
    }

    pub fn new_empty(state: Arc<StateDelta>) -> Self {
        /*
        Self::new_impl(Inner {
            parent_state: state.clone(),
            state_authenticator: state,
            state_updates_before_last_checkpoint: None,
            state_checkpoint_hashes: vec![],
        })

         */
        todo!() // FIXME(aldenhu)
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(Arc::new(StateDelta::new_empty()))
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        Self::new_empty(self.state_auth.clone())
    }
}

#[derive(Debug, Default)]
pub struct Inner {
    /// FIXME(aldenhu): see if it's useful
    pub parent_auth: StateAuthenticator,
    pub last_checkpoint_auth: Option<StateAuthenticator>,
    pub state_auth: StateAuthenticator,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
}
