// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::Vote, NodeId};
use crate::dag::{CertifiedNode, Node};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_types::ledger_info::LedgerInfoWithSignatures;

#[derive(Clone)]
pub struct CommitEvent {
    node_id: NodeId,
    parents: Vec<Author>,
    failed_authors: Vec<Author>,
}

impl CommitEvent {
    pub fn new(node_id: NodeId, parents: Vec<Author>, failed_authors: Vec<Author>) -> Self {
        CommitEvent {
            node_id,
            parents,
            failed_authors,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.node_id.epoch()
    }

    pub fn round(&self) -> Round {
        self.node_id.round()
    }

    pub fn author(&self) -> &Author {
        self.node_id.author()
    }

    pub fn parents(&self) -> &[Author] {
        &self.parents
    }

    pub fn failed_authors(&self) -> &[Author] {
        &self.failed_authors
    }
}

pub trait DAGStorage: Send + Sync {
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()>;

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>>;

    fn delete_pending_node(&self) -> anyhow::Result<()>;

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()>;

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>>;

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()>;

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()>;

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>>;

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()>;

    fn get_latest_k_committed_events(&self, k: u64) -> anyhow::Result<Vec<CommitEvent>>;

    fn get_latest_ledger_info(&self) -> anyhow::Result<LedgerInfoWithSignatures>;
}
