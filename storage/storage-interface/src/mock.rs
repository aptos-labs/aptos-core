// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{DbReader, DbWriter};
use anyhow::Result;
use aptos_types::{
    proof::SparseMerkleProofExt, state_store::state_key::StateKey, transaction::Version,
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
        _state_key: &StateKey,
        _version: Version,
    ) -> Result<SparseMerkleProofExt> {
        Ok(SparseMerkleProofExt::new(None, vec![]))
    }
}

impl DbWriter for MockDbReaderWriter {}
