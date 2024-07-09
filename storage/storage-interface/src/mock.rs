// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{errors::AptosDbError, DbReader, DbWriter, Result};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    event::EventHandle,
    proof::SparseMerkleProofExt,
    state_store::{state_key::inner::StateKeyInner, state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use move_core_types::move_resource::MoveResource;

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
        _root_depth: usize,
    ) -> Result<SparseMerkleProofExt> {
        Ok(SparseMerkleProofExt::new(None, vec![]))
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        _: Version,
    ) -> Result<Option<StateValue>> {
        match state_key.inner() {
            StateKeyInner::AccessPath(access_path) => {
                if access_path.path == AccountResource::resource_path() {
                    let account_resource = AccountResource::new(
                        0,
                        vec![],
                        EventHandle::random(0),
                        EventHandle::random(0),
                    );
                    let value = bcs::to_bytes(&account_resource).unwrap();
                    Ok(Some(value.into()))
                } else {
                    Ok(None)
                }
            },
            StateKeyInner::Raw(raw_key) => Ok(Some(StateValue::from(raw_key.to_owned()))),
            _ => Err(AptosDbError::Other(format!(
                "Not supported state key type {:?}",
                state_key
            ))),
        }
    }

    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        Ok(self
            .get_state_value_by_version(state_key, version)?
            .map(|value| (version, value)))
    }

    fn get_block_info_by_height(
        &self,
        height: u64,
    ) -> Result<(Version, Version, aptos_types::account_config::NewBlockEvent)> {
        Ok((
            0,
            1,
            aptos_types::account_config::NewBlockEvent::new(
                AccountAddress::ONE,
                0,
                0,
                height,
                vec![],
                AccountAddress::TWO,
                vec![],
                0,
            ),
        ))
    }

    fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue> {
        Ok(HashValue::zero())
    }
}

impl DbWriter for MockDbReaderWriter {}
