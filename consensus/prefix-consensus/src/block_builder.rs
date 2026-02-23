// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Block builder for Prefix Consensus.
//!
//! Constructs a `Block` from SPC's v_high output by aggregating committed
//! proposals' payloads in ranking order.

use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, Round},
};
use aptos_crypto::HashValue;
use aptos_types::validator_txn::ValidatorTransaction;
use std::collections::HashMap;

/// Build a `Block` from the SPC v_high output for a slot.
///
/// `ranking` is the ordered list of validators for this slot (length n).
/// `v_high` is the SPC output — a variable-length prefix (length <= n) of hashes,
/// where `HashValue::zero()` represents bottom (no proposal received).
///
/// For each non-bottom entry in v_high, looks up the corresponding payload in
/// `payload_map`. Entries whose payload is missing are skipped (this should not
/// happen if payload resolution in Phase 7 succeeded).
///
/// The block's author is the highest-ranked validator with a committed proposal
/// (first non-bottom entry in v_high). This is deterministic since all nodes
/// agree on v_high and ranking. Falls back to `ranking[0]` if v_high is all-bottom.
///
/// Returns a `Block` with the aggregated payload and metadata.
pub fn build_block_from_v_high(
    epoch: u64,
    round: Round,
    timestamp_usecs: u64,
    ranking: &[Author],
    v_high: &[HashValue],
    payload_map: &HashMap<HashValue, Payload>,
    parent_block_id: HashValue,
    validator_txns: Vec<ValidatorTransaction>,
) -> Block {
    let mut authors = Vec::new();
    let mut proposal_hashes = Vec::new();
    let mut aggregated_payload = Payload::DirectMempool(vec![]);

    // zip terminates at the shorter iterator, correctly skipping validators
    // beyond v_high's length (excluded validators whose proposals are not committed)
    for (hash, ranked_author) in v_high.iter().zip(ranking.iter()) {
        // HashValue::zero() is the bottom marker — no proposal at this position
        if *hash != HashValue::zero() {
            let payload = payload_map
                .get(hash)
                .expect("Payload missing for committed hash — payload resolution bug");
            authors.push(*ranked_author);
            proposal_hashes.push(*hash);
            aggregated_payload = aggregated_payload.extend(payload.clone());
        }
    }

    // Block author is the highest-ranked validator with a committed proposal.
    // This is deterministic: all nodes agree on v_high (from SPC) and ranking.
    // Fallback to ranking[0] if v_high is all-bottom (shouldn't happen in practice
    // due to SPC's validity property).
    let block_author = authors.first().copied().unwrap_or(ranking[0]);

    Block::new_for_prefix_consensus(
        epoch,
        round,
        timestamp_usecs,
        validator_txns,
        aggregated_payload,
        block_author,
        authors,
        proposal_hashes,
        parent_block_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::block_data::BlockType;
    use aptos_types::account_address::AccountAddress;

    fn make_author(byte: u8) -> Author {
        AccountAddress::new([byte; AccountAddress::LENGTH])
    }

    /// Create an empty DirectMempool payload with a unique random hash.
    fn make_empty_payload() -> (Payload, HashValue) {
        (Payload::DirectMempool(vec![]), HashValue::random())
    }

    #[test]
    fn test_build_block_all_non_bot() {
        let ranking = vec![make_author(1), make_author(2), make_author(3), make_author(4)];

        let (p1, h1) = make_empty_payload();
        let (p2, h2) = make_empty_payload();
        let (p3, h3) = make_empty_payload();
        let (p4, h4) = make_empty_payload();

        let v_high = vec![h1, h2, h3, h4];
        let mut payload_map = HashMap::new();
        payload_map.insert(h1, p1);
        payload_map.insert(h2, p2);
        payload_map.insert(h3, p3);
        payload_map.insert(h4, p4);

        let block = build_block_from_v_high(
            1, 10, 1000,
            &ranking, &v_high, &payload_map,
            HashValue::random(), vec![],
        );

        assert_eq!(block.epoch(), 1);
        assert_eq!(block.round(), 10);
        assert_eq!(block.timestamp_usecs(), 1000);
        // Block author = highest-ranked committed author = ranking[0]
        assert_eq!(block.author(), Some(make_author(1)));

        // All 4 proposals committed
        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { authors, proposal_hashes, .. } => {
                assert_eq!(authors.len(), 4);
                assert_eq!(proposal_hashes.len(), 4);
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
        assert!(block.payload().is_some());
    }

    #[test]
    fn test_build_block_partial() {
        let ranking = vec![make_author(1), make_author(2), make_author(3), make_author(4)];

        let (p1, h1) = make_empty_payload();
        let (p3, h3) = make_empty_payload();

        // Positions 1 and 3 are bottom (no proposal)
        let v_high = vec![h1, HashValue::zero(), h3, HashValue::zero()];
        let mut payload_map = HashMap::new();
        payload_map.insert(h1, p1);
        payload_map.insert(h3, p3);

        let block = build_block_from_v_high(
            1, 10, 1000,
            &ranking, &v_high, &payload_map,
            HashValue::random(), vec![],
        );

        // Block author = first committed = make_author(1) (position 0)
        assert_eq!(block.author(), Some(make_author(1)));

        // Only 2 proposals committed (positions 0 and 2)
        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { authors, proposal_hashes, .. } => {
                assert_eq!(authors.len(), 2);
                assert_eq!(authors, &vec![make_author(1), make_author(3)]);
                assert_eq!(proposal_hashes, &vec![h1, h3]);
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
    }

    #[test]
    fn test_build_block_empty_v_high() {
        let ranking = vec![make_author(1), make_author(2), make_author(3), make_author(4)];
        let v_high = vec![
            HashValue::zero(), HashValue::zero(), HashValue::zero(), HashValue::zero(),
        ];
        let payload_map = HashMap::new();

        let block = build_block_from_v_high(
            1, 10, 1000,
            &ranking, &v_high, &payload_map,
            HashValue::random(), vec![],
        );

        // All-bottom v_high: author falls back to ranking[0]
        assert_eq!(block.author(), Some(make_author(1)));

        // No proposals committed — empty payload, no authors
        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { authors, proposal_hashes, payload, .. } => {
                assert!(authors.is_empty());
                assert!(proposal_hashes.is_empty());
                assert!(payload.is_empty());
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
    }

    #[test]
    fn test_build_block_short_v_high() {
        // v_high has length 2 with ranking of 4 validators
        let ranking = vec![make_author(1), make_author(2), make_author(3), make_author(4)];

        let (p1, h1) = make_empty_payload();
        let (p2, h2) = make_empty_payload();

        let v_high = vec![h1, h2]; // Only 2 entries — validators 3 and 4 excluded by zip
        let mut payload_map = HashMap::new();
        payload_map.insert(h1, p1);
        payload_map.insert(h2, p2);

        let block = build_block_from_v_high(
            1, 10, 1000,
            &ranking, &v_high, &payload_map,
            HashValue::random(), vec![],
        );

        // Only first 2 proposals committed
        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { authors, proposal_hashes, .. } => {
                assert_eq!(authors.len(), 2);
                assert_eq!(authors, &vec![make_author(1), make_author(2)]);
                assert_eq!(proposal_hashes.len(), 2);
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
    }

    #[test]
    fn test_build_block_ordering() {
        let a1 = make_author(1);
        let a2 = make_author(2);
        let a3 = make_author(3);
        let ranking = vec![a1, a2, a3];

        let (p1, h1) = make_empty_payload();
        let (p2, h2) = make_empty_payload();
        let (p3, h3) = make_empty_payload();

        let v_high = vec![h1, h2, h3];
        let mut payload_map = HashMap::new();
        payload_map.insert(h1, p1);
        payload_map.insert(h2, p2);
        payload_map.insert(h3, p3);

        let block = build_block_from_v_high(
            1, 10, 1000,
            &ranking, &v_high, &payload_map,
            HashValue::random(), vec![],
        );

        // Block author = first committed author = a1
        assert_eq!(block.author(), Some(a1));

        // Verify authors and proposal_hashes follow ranking order
        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { authors, proposal_hashes, .. } => {
                assert_eq!(authors, &vec![a1, a2, a3]);
                assert_eq!(proposal_hashes, &vec![h1, h2, h3]);
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
    }

    #[test]
    fn test_build_block_metadata() {
        let (p1, h1) = make_empty_payload();
        let parent_id = HashValue::random();
        let mut payload_map = HashMap::new();
        payload_map.insert(h1, p1);

        let block = build_block_from_v_high(
            7, 42, 999_000,
            &[make_author(5), make_author(6)],
            &[h1, HashValue::zero()],
            &payload_map,
            parent_id,
            vec![],
        );

        assert_eq!(block.epoch(), 7);
        assert_eq!(block.round(), 42);
        assert_eq!(block.timestamp_usecs(), 999_000);
        // Block author = first committed = make_author(5)
        assert_eq!(block.author(), Some(make_author(5)));
        assert_eq!(block.parent_id(), parent_id);

        match block.block_data().block_type() {
            BlockType::PrefixConsensusBlock { failed_authors, .. } => {
                assert!(failed_authors.is_empty());
            },
            _ => panic!("Expected PrefixConsensusBlock"),
        }
    }
}
