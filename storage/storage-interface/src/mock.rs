// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{errors::VelorDbError, DbReader, DbWriter, Result};
use velor_crypto::HashValue;
use velor_types::{
    proof::SparseMerkleProofExt,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        state_value::StateValue,
    },
    transaction::Version,
};

/// This is a mock of the DbReaderWriter in tests.
pub struct MockDbReaderWriter;

impl DbReader for MockDbReaderWriter {
    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        // return a dummy version for tests
        Ok(Some(1))
    }

    fn get_state_proof_by_version_ext(
        &self,
        _key_hash: &HashValue,
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
            StateKeyInner::AccessPath(..) => Ok(None),
            StateKeyInner::Raw(raw_key) => Ok(Some(StateValue::from(raw_key.to_owned()))),
            _ => Err(VelorDbError::Other(format!(
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
}

impl DbWriter for MockDbReaderWriter {}
