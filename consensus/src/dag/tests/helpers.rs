// Copyright © Aptos Foundation

use crate::{
    dag::types::{CertifiedNode, Extensions, Node, NodeCertificate},
    payload_manager::TPayloadManager,
};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_types::aggregate_signature::AggregateSignature;

pub(super) const TEST_DAG_WINDOW: u64 = 5;

pub(super) struct MockPayloadManager {}

impl TPayloadManager for MockPayloadManager {
    fn prefetch_payload_data(&self, _payload: &Payload, _timestamp: u64) {}
}

pub(crate) fn new_certified_node(
    round: Round,
    author: Author,
    parents: Vec<NodeCertificate>,
) -> CertifiedNode {
    let node = Node::new(
        1,
        round,
        author,
        0,
        vec![],
        Payload::empty(false),
        parents,
        Extensions::empty(),
    );
    CertifiedNode::new(node, AggregateSignature::empty())
}

pub(crate) fn new_node(
    round: Round,
    timestamp: u64,
    author: Author,
    parents: Vec<NodeCertificate>,
) -> Node {
    Node::new(
        0,
        round,
        author,
        timestamp,
        vec![],
        Payload::empty(false),
        parents,
        Extensions::empty(),
    )
}

/// Generate certified nodes for dag given the virtual dag
pub(crate) fn generate_dag_nodes(
    dag: &[Vec<Option<Vec<bool>>>],
    validators: &[Author],
) -> Vec<Vec<Option<CertifiedNode>>> {
    let mut nodes = vec![];
    let mut previous_round: Vec<Option<CertifiedNode>> = vec![];
    for (round, round_nodes) in dag.iter().enumerate() {
        let mut nodes_at_round = vec![];
        for (idx, author) in validators.iter().enumerate() {
            if let Some(bitmask) = &round_nodes[idx] {
                // the bitmask is compressed (without the holes), we need to flatten the previous round nodes
                // to match the index
                let parents: Vec<_> = previous_round
                    .iter()
                    .flatten()
                    .enumerate()
                    .filter(|(idx, _)| *bitmask.get(*idx).unwrap_or(&false))
                    .map(|(_, node)| {
                        NodeCertificate::new(node.metadata().clone(), AggregateSignature::empty())
                    })
                    .collect();
                if round > 1 {
                    assert_eq!(parents.len(), validators.len() * 2 / 3 + 1);
                }
                nodes_at_round.push(Some(new_certified_node(
                    (round + 1) as u64,
                    *author,
                    parents,
                )));
            } else {
                nodes_at_round.push(None);
            }
        }
        previous_round = nodes_at_round.clone();
        nodes.push(nodes_at_round);
    }
    nodes
}
