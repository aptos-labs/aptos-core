// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared state-value snapshot chunk helpers, generic over the backing
//! Jellyfish Merkle store. The JMT walk and range-proof assembly are identical
//! across snapshot kinds; only the value lookup differs.

#![forbid(unsafe_code)]

use aptos_crypto::hash::CryptoHash;
use aptos_jellyfish_merkle::{iterator::JellyfishMerkleIterator, JellyfishMerkleTree, TreeReader};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{
        state_key::StateKey,
        state_value::{StateValue, StateValueChunkWithProof},
    },
    transaction::Version,
};
use std::sync::Arc;

/// Walks the JMT at `version` from `start_idx` and yields `(StateKey, StateValue)`
/// for each live leaf, resolving the value via `value_for(key, leaf_version)`.
pub(crate) fn jmt_leaves_with_values<R, F>(
    merkle_db: Arc<R>,
    version: Version,
    start_idx: usize,
    value_for: F,
) -> Result<impl Iterator<Item = Result<(StateKey, StateValue)>> + Send + Sync + use<R, F>>
where
    R: TreeReader<StateKey> + Send + Sync + 'static,
    F: Fn(&StateKey, Version) -> Result<StateValue> + Send + Sync + 'static,
{
    Ok(
        JellyfishMerkleIterator::new_by_index(merkle_db, version, start_idx)?.map(move |res| {
            res.and_then(|(_hashed_key, (key, leaf_version))| {
                let value = value_for(&key, leaf_version)?;
                Ok((key, value))
            })
        }),
    )
}

/// Reads up to `chunk_size` live leaves from `first_index` (resolving values via
/// `value_for`) and stamps them with a range proof against the store's root at
/// `version`. The caller re-requests any remainder.
pub(crate) fn value_chunk_with_proof<R, F>(
    merkle_db: Arc<R>,
    version: Version,
    first_index: usize,
    chunk_size: usize,
    value_for: F,
) -> Result<StateValueChunkWithProof>
where
    R: TreeReader<StateKey> + Send + Sync + 'static,
    F: Fn(&StateKey, Version) -> Result<StateValue> + Send + Sync + 'static,
{
    let raw_values =
        jmt_leaves_with_values(Arc::clone(&merkle_db), version, first_index, value_for)?
            .take(chunk_size)
            .collect::<Result<Vec<_>>>()?;
    build_value_chunk_proof(merkle_db.as_ref(), version, first_index, raw_values)
}

/// Assembles a [`StateValueChunkWithProof`] for `raw_values`: a range proof for
/// the rightmost key against the store's root at `version`. The caller is
/// responsible for any byte/time bounding of `raw_values`.
pub(crate) fn build_value_chunk_proof<R>(
    merkle_db: &R,
    version: Version,
    first_index: usize,
    raw_values: Vec<(StateKey, StateValue)>,
) -> Result<StateValueChunkWithProof>
where
    R: TreeReader<StateKey> + Sync,
{
    if raw_values.is_empty() {
        return Err(AptosDbError::Other(format!(
            "State value chunk starting at {first_index} is empty"
        )));
    }
    let last_index = (first_index + raw_values.len() - 1) as u64;
    let first_key = raw_values.first().expect("checked non-empty").0.hash();
    let last_key = raw_values.last().expect("checked non-empty").0.hash();
    let tree = JellyfishMerkleTree::<R, StateKey>::new(merkle_db);
    let proof = tree.get_range_proof(last_key, version)?;
    let root_hash = tree.get_root_hash(version)?;
    Ok(StateValueChunkWithProof {
        first_index: first_index as u64,
        last_index,
        first_key,
        last_key,
        raw_values,
        proof,
        root_hash,
    })
}
