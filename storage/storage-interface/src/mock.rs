// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{DbReader, DbWriter};
use anyhow::{anyhow, Result};
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    account_state::AccountState,
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use move_deps::move_core_types::move_resource::MoveResource;

/// This is a mock of the DbReaderWriter in tests.
pub struct MockDbReaderWriter;

impl DbReader for MockDbReaderWriter {
    fn get_latest_state_value(&self, state_key: StateKey) -> Result<Option<StateValue>> {
        match state_key {
            StateKey::AccessPath(access_path) => {
                let account_state = get_mock_account_state();
                Ok(account_state
                    .get(&access_path.path)
                    .cloned()
                    .map(StateValue::from))
            }
            _ => Err(anyhow!("Not supported state key type {:?}", state_key)),
        }
    }

    fn get_latest_version_option(&self) -> Result<Option<Version>> {
        // return a dummy version for tests
        Ok(Some(1))
    }

    fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        _: Version,
    ) -> Result<(Option<StateValue>, SparseMerkleProof<StateValue>)> {
        // dummy proof which is not used
        Ok((
            self.get_latest_state_value(state_key.clone()).unwrap(),
            SparseMerkleProof::new(None, vec![]),
        ))
    }
}

fn get_mock_account_state() -> AccountState {
    let account_resource = AccountResource::new(0, vec![], AccountAddress::random());

    let mut account_state = AccountState::default();
    account_state.insert(
        AccountResource::resource_path(),
        bcs::to_bytes(&account_resource).unwrap(),
    );
    account_state
}

impl DbWriter for MockDbReaderWriter {}
