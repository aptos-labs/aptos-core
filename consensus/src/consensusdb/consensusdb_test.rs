// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use self::schema::dag::NodeSchema;
use super::*;
use crate::dag::{CertifiedNode, Extensions, Node, Vote};
use velor_consensus_types::{
    block::block_test_utils::certificate_for_genesis,
    common::{Author, Payload},
};
use velor_crypto::bls12381::Signature;
use velor_temppath::TempPath;
use velor_types::aggregate_signature::AggregateSignature;
use std::{collections::HashMap, hash::Hash};

#[test]
fn test_put_get() {
    let tmp_dir = TempPath::new();
    let db = ConsensusDB::new(&tmp_dir);

    let block = Block::make_genesis_block();
    let blocks = vec![block];

    assert_eq!(db.get_all::<BlockSchema>().unwrap().len(), 0);
    assert_eq!(db.get_all::<QCSchema>().unwrap().len(), 0);

    let qcs = vec![certificate_for_genesis()];
    db.save_blocks_and_quorum_certificates(blocks.clone(), qcs.clone())
        .unwrap();

    assert_eq!(db.get_all::<BlockSchema>().unwrap().len(), 1);
    assert_eq!(db.get_all::<QCSchema>().unwrap().len(), 1);

    let tc = vec![0u8, 1, 2];
    db.save_highest_2chain_timeout_certificate(tc.clone())
        .unwrap();

    let vote = vec![2u8, 1, 0];
    db.save_vote(vote.clone()).unwrap();

    let (vote_1, tc_1, blocks_1, qc_1) = db.get_data().unwrap();
    assert_eq!(blocks, blocks_1);
    assert_eq!(qcs, qc_1);
    assert_eq!(Some(tc), tc_1);
    assert_eq!(Some(vote), vote_1);

    db.delete_highest_2chain_timeout_certificate().unwrap();
    db.delete_last_vote_msg().unwrap();
    assert!(db
        .get_highest_2chain_timeout_certificate()
        .unwrap()
        .is_none());
    assert!(db.get_last_vote().unwrap().is_none());
}

#[test]
fn test_delete_block_and_qc() {
    let tmp_dir = TempPath::new();
    let db = ConsensusDB::new(&tmp_dir);

    assert_eq!(db.get_all::<BlockSchema>().unwrap().len(), 0);
    assert_eq!(db.get_all::<QCSchema>().unwrap().len(), 0);

    let blocks = vec![Block::make_genesis_block()];
    let block_id = blocks[0].id();

    let qcs = vec![certificate_for_genesis()];
    let qc_id = qcs[0].certified_block().id();

    db.save_blocks_and_quorum_certificates(blocks, qcs).unwrap();
    assert_eq!(db.get_all::<BlockSchema>().unwrap().len(), 1);
    assert_eq!(db.get_all::<QCSchema>().unwrap().len(), 1);

    // Start to delete
    db.delete_blocks_and_quorum_certificates(vec![block_id, qc_id])
        .unwrap();
    assert_eq!(db.get_all::<BlockSchema>().unwrap().len(), 0);
    assert_eq!(db.get_all::<QCSchema>().unwrap().len(), 0);
}

fn test_dag_type<S: Schema<Key = K>, K: Eq + Hash>(key: S::Key, value: S::Value, db: &ConsensusDB) {
    db.put::<S>(&key, &value).unwrap();
    let mut from_db: HashMap<K, S::Value> = db.get_all::<S>().unwrap().into_iter().collect();
    assert_eq!(from_db.len(), 1);
    let value_from_db = from_db.remove(&key).unwrap();
    assert_eq!(value, value_from_db);
    db.delete::<S>(vec![key]).unwrap();
    assert_eq!(db.get_all::<S>().unwrap().len(), 0);
}

#[test]
fn test_dag() {
    let tmp_dir = TempPath::new();
    let db = ConsensusDB::new(&tmp_dir);

    let node = Node::new(
        1,
        1,
        Author::random(),
        123,
        vec![],
        Payload::empty(false, true),
        vec![],
        Extensions::empty(),
    );
    test_dag_type::<NodeSchema, <NodeSchema as Schema>::Key>((), node.clone(), &db);

    let certified_node = CertifiedNode::new(node.clone(), AggregateSignature::empty());
    test_dag_type::<CertifiedNodeSchema, <CertifiedNodeSchema as Schema>::Key>(
        certified_node.digest(),
        certified_node,
        &db,
    );

    let vote = Vote::new(node.metadata().clone(), Signature::dummy_signature());
    test_dag_type::<DagVoteSchema, <DagVoteSchema as Schema>::Key>(node.id(), vote, &db);
}
