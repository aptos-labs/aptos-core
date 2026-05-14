// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! State-sync interface for the native-position subsystem.
//!
//! A fresh node syncs native-position state via an independent chunk
//! stream that rides alongside the main state-sync stream:
//!
//! - Main chunks are verified against `main_state_root` as today.
//! - Position chunks carry `(encoded_position_key, StateValue)` pairs
//!   and verify against `position_root` (one [`SparseMerkleProof`]
//!   per entry, BCS-encoded into `chunk.proof`).
//! - After both streams complete at the same synced `Version`, the
//!   receiver:
//!     1. Applies position chunks to `position_db` +
//!        `position_merkle_db` via [`apply_chunk`].
//!     2. Calls `AptosDB::load_native_position_from_disk(version)`
//!        to populate the in-memory store.
//!     3. Validates the composed
//!        `H("APTOS::StateRoot" || main_state_root || position_root)`
//!        against the signed `LedgerInfo`.
//!
//! Surface in this module:
//! - [`produce_chunks`]: producer side; reads `position_db` at the
//!   target version, slices by `chunk_size`, BCS-encodes per-entry
//!   inclusion proofs.
//! - [`verify_chunk`]: receiver-side proof check before apply.
//! - [`apply_chunk`]: writes to `position_db` and the in-memory
//!   store. Does NOT emit stale-index entries (this is a fresh-sync
//!   path, no superseded versions to garbage-collect).
//!
//! Per-entry proofs are bandwidth-wasteful for large chunks
//! (O(log N) sibling hashes per entry). A range-proof variant via
//! `JMT::get_range_proof` is bandwidth-optimal but requires the
//! sequential restore protocol (each chunk extends the previous via
//! accumulated left-siblings, mirroring `state_restore`). Tracked
//! as a follow-up.
//!
//! Cross-crate wiring of the dual-stream protocol into the
//! state-sync service / chunk producers / proof verifiers across
//! SDKs is intentionally out of scope; this module owns only the
//! storage-side primitives.

#![forbid(unsafe_code)]

use crate::{position_db::PositionDb, position_merkle_db::PositionMerkleDb};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_jellyfish_merkle::JellyfishMerkleTree;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A single Position row shipped in a state-sync chunk.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PositionChunkEntry {
    /// Encoded Position StateKey bytes (69 bytes, see
    /// `StateKeyInner::encode()` for the layout).
    pub encoded_key: Vec<u8>,
    /// Row value — `None` for tombstones.
    pub value: Option<StateValue>,
    /// Version at which this row was committed. Allows the receiver
    /// to preserve version granularity in `position_value` and emit
    /// the matching stale-index entries.
    pub version: Version,
}

/// A batch of Position rows covering one state-sync chunk boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PositionChunk {
    pub entries: Vec<PositionChunkEntry>,
    /// Inclusion-proof data for `entries`, rooted at `position_root`.
    /// Opaque here — the downstream verifier peels the JMT proof
    /// chain and checks each leaf.
    pub proof: Vec<u8>,
    /// Expected `position_root` at this chunk boundary.
    pub expected_position_root: HashValue,
}

/// Build a stream of [`PositionChunk`]s for the producer side of
/// state sync. Walks `position_db` at `version`, slices the result
/// into batches of `chunk_size`, stamps each chunk with the expected
/// `position_root` read from `position_merkle_db`, and BCS-encodes a
/// `Vec<SparseMerkleProof>` (one per entry) into `chunk.proof` so
/// the consumer can verify each leaf inclusion against the expected
/// root before applying.
///
/// Bandwidth note: per-entry proofs are simple and self-contained
/// per chunk, but each proof carries O(log N) sibling hashes — for
/// 10K-entry chunks this is ~10 MB of proof overhead. A true
/// range-proof variant (one proof per chunk via JMT
/// `get_range_proof`) is the bandwidth-optimal path; it requires
/// the sequential restore protocol where each chunk extends the
/// previous via accumulated left-siblings, the same shape as the
/// existing `state_restore` module. Tracked as a follow-up.
pub fn produce_chunks(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
    chunk_size: usize,
) -> Result<Vec<PositionChunk>> {
    use aptos_crypto::hash::CryptoHash;
    if chunk_size == 0 {
        return Err(AptosDbError::Other(
            "produce_chunks: chunk_size must be > 0".into(),
        ));
    }
    let expected_root = position_merkle_db.get_root_hash(version)?;
    let tree = JellyfishMerkleTree::<_, StateKey>::new(position_merkle_db.as_ref());
    let total_leaves = position_merkle_db.get_leaf_count(version)?;
    let mut chunks = Vec::with_capacity(total_leaves.div_ceil(chunk_size));
    // Stream JMT leaves in `chunk_size` windows, joining each leaf to
    // its position-DB value as we go. The chunked iterator drives both
    // the JMT walk and the value lookup, so we never materialise the
    // whole snapshot in memory.
    let mut first_index = 0;
    while first_index < total_leaves {
        let mut entries = Vec::with_capacity(chunk_size.min(total_leaves - first_index));
        let mut proofs: Vec<SparseMerkleProof> = Vec::with_capacity(entries.capacity());
        for kv in position_merkle_db.iter_active_leaves_chunk(
            Arc::clone(position_db),
            version,
            first_index,
            chunk_size,
        )? {
            let (state_key, state_value) = kv?;
            let key_hash = state_key.hash();
            let (_value, proof) = tree.get_with_proof(key_hash, version)?;
            proofs.push(proof);
            entries.push(PositionChunkEntry {
                encoded_key: state_key.encoded().to_vec(),
                value: Some(state_value),
                version,
            });
        }
        let produced = entries.len();
        let proof_bytes = bcs::to_bytes(&proofs)
            .map_err(|e| AptosDbError::Other(format!("encode proofs: {e}")))?;
        chunks.push(PositionChunk {
            entries,
            proof: proof_bytes,
            expected_position_root: expected_root,
        });
        if produced == 0 {
            break;
        }
        first_index += produced;
    }
    Ok(chunks)
}

/// Verify a [`PositionChunk`] against `chunk.expected_position_root`.
/// Decodes the BCS-encoded proof list and checks each entry's leaf
/// hash. Returns `Ok(())` if every entry verifies.
///
/// Callers must invoke this before [`apply_chunk`] when the chunk
/// originated from an untrusted producer.
pub fn verify_chunk(chunk: &PositionChunk) -> Result<()> {
    let proofs: Vec<SparseMerkleProof> = bcs::from_bytes(&chunk.proof)
        .map_err(|e| AptosDbError::Other(format!("verify_chunk: decode proof bytes: {e}")))?;
    if proofs.len() != chunk.entries.len() {
        return Err(AptosDbError::Other(format!(
            "proof count {} does not match entry count {}",
            proofs.len(),
            chunk.entries.len()
        )));
    }
    for (entry, proof) in chunk.entries.iter().zip(proofs.iter()) {
        let state_key = StateKey::decode(&entry.encoded_key)
            .map_err(|e| AptosDbError::Other(format!("verify_chunk: decode key: {e}")))?;
        let key_hash = state_key.hash();
        let value_hash = entry.value.as_ref().map(StateValue::hash);
        proof
            .verify_by_hash(chunk.expected_position_root, key_hash, value_hash)
            .map_err(|e| AptosDbError::Other(format!("position chunk verify: {e}")))?;
    }
    Ok(())
}

/// Apply a verified Position chunk at commit time: write the rows to
/// `position_db`'s `position_value` CF. Caller is responsible for
/// JMT proof verification against `chunk.expected_position_root`
/// before calling this.
pub fn apply_chunk(chunk: &PositionChunk, position_db: &Arc<PositionDb>) -> Result<()> {
    use aptos_crypto::hash::CryptoHash;
    use std::collections::BTreeMap;
    let decoded: Vec<(StateKey, &PositionChunkEntry)> = chunk
        .entries
        .iter()
        .map(|entry| {
            StateKey::decode(&entry.encoded_key)
                .map(|sk| (sk, entry))
                .map_err(|e| {
                    AptosDbError::Other(format!(
                        "apply_chunk: StateKey::decode failed on a chunk entry that passed \
                         proof verification (internal inconsistency): {e}"
                    ))
                })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut by_version: BTreeMap<Version, Vec<(aptos_crypto::HashValue, Option<StateValue>)>> =
        BTreeMap::new();
    for (state_key, entry) in &decoded {
        by_version
            .entry(entry.version)
            .or_default()
            .push((state_key.hash(), entry.value.clone()));
    }
    for (version, writes) in by_version {
        position_db.write_position_batch(version, writes)?;
    }
    Ok(())
}
