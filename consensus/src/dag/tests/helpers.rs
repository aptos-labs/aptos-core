// Copyright © Aptos Foundation

use crate::dag::types::{CertifiedNode, Node, NodeCertificate};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_types::aggregate_signature::AggregateSignature;

pub(crate) fn new_certified_node(
    round: Round,
    author: Author,
    parents: Vec<NodeCertificate>,
) -> CertifiedNode {
    let node = Node::new(1, round, author, 0, Payload::empty(false), parents);
    CertifiedNode::new(node, AggregateSignature::empty())
}

pub(crate) fn new_node(
    round: Round,
    timestamp: u64,
    author: Author,
    parents: Vec<NodeCertificate>,
) -> Node {
    Node::new(0, round, author, timestamp, Payload::empty(false), parents)
}
