// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{macros::delegate_read, read_delegation::ReadDelegation};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_types::{
    proof::SparseMerkleProofExt,
    state_store::{
        state_key::StateKey, state_storage_usage::StateStorageUsage, state_value::StateValue,
    },
    transaction::Version,
};

pub trait StateReader: Send + Sync + ReadDelegation {
    delegate_read!(
        /// Returns the latest state snapshot strictly before `next_version` if any.
        fn get_state_snapshot_before(
            &self,
            next_version: Version,
        ) -> Result<Option<(Version, HashValue)>>;

        /// Returns the latest state value of the given key up to the given version.
        fn get_state_value_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<Option<StateValue>>;

        /// Returns the latest state value and its corresponding version when it's of the given key
        /// up to the given version.
        fn get_state_value_with_version_by_version(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<Option<(Version, StateValue)>>;

        /// Returns the proof of the given state key and version.
        fn get_state_proof_by_version_ext(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<SparseMerkleProofExt>;

        /// Returns the state value with proof given the state key and version.
        fn get_state_value_with_proof_by_version_ext(
            &self,
            state_key: &StateKey,
            version: Version,
        ) -> Result<(Option<StateValue>, SparseMerkleProofExt)>;

        /// Returns state storage usage at the version.
        fn get_state_storage_usage(&self, version: Option<Version>) -> Result<StateStorageUsage>;
    );
}
