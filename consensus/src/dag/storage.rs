// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::Vote, NodeId};
use crate::{
    consensusdb::ConsensusDB,
    dag::{CertifiedNode, Node},
};
use anyhow::Ok;
use aptos_crypto::HashValue;
use std::collections::HashMap;

pub trait DAGStorage {
    fn save_node(&self, node: &Node) -> anyhow::Result<()>;

    fn delete_node(&self, digest: HashValue) -> anyhow::Result<()>;

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()>;

    fn get_votes(&self) -> anyhow::Result<HashMap<NodeId, Vote>>;

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()>;

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()>;

    fn get_certified_nodes(&self) -> anyhow::Result<HashMap<HashValue, CertifiedNode>>;

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()>;
}

impl DAGStorage for ConsensusDB {
    fn save_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.save_node(node)?)
    }

    fn delete_node(&self, digest: HashValue) -> anyhow::Result<()> {
        Ok(self.delete_node(digest)?)
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        Ok(self.save_dag_vote(node_id, vote)?)
    }

    fn get_votes(&self) -> anyhow::Result<HashMap<NodeId, Vote>> {
        Ok(self.get_dag_votes()?)
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete_dag_votes(node_ids)?)
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        Ok(self.save_certified_node(node)?)
    }

    fn get_certified_nodes(&self) -> anyhow::Result<HashMap<HashValue, CertifiedNode>> {
        Ok(self.get_certified_nodes()?)
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        Ok(self.delete_certified_nodes(digests)?)
    }
}
