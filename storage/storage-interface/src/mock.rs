// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{DbReader, DbWriter};
use anyhow::{anyhow, Result};
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::AccountResource;
use aptos_types::account_state::AccountState;
use aptos_types::event::EventHandle;
use aptos_types::state_store::state_value::StateValue;
use aptos_types::{
    proof::SparseMerkleProofExt, state_store::state_key::StateKey, transaction::Version,
};
use move_deps::move_core_types::move_resource::MoveResource;

/// This is a mock of the DbReaderWriter in tests.
pub struct MockDbReaderWriter;

impl DbReader for MockDbReaderWriter {
    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        // return a dummy version for tests
        Ok(Some(1))
    }

    fn get_state_proof_by_version_ext(
        &self,
        _state_key: &StateKey,
        _version: Version,
    ) -> Result<SparseMerkleProofExt> {
        Ok(SparseMerkleProofExt::new(None, vec![]))
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        _: Version,
    ) -> Result<Option<StateValue>> {
        match state_key {
            StateKey::AccessPath(access_path) => {
                let account_state = get_mock_account_state();
                Ok(account_state
                    .get(&access_path.path)
                    .cloned()
                    .map(StateValue::from))
            }
            StateKey::Raw(raw_key) => Ok(Some(StateValue::from(raw_key.to_owned()))),
            _ => Err(anyhow!("Not supported state key type {:?}", state_key)),
        }
    }
}

fn get_mock_account_state() -> AccountState {
    let account_resource =
        AccountResource::new(0, vec![], EventHandle::random(0), EventHandle::random(0));

    AccountState::new(
        AccountAddress::random(),
        std::collections::BTreeMap::from([(
            AccountResource::resource_path(),
            bcs::to_bytes(&account_resource).unwrap(),
        )]),
    )
}

impl DbWriter for MockDbReaderWriter {}
