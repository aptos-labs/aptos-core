// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::Vote, NodeId};
use crate::{
    consensusdb::{
        CertifiedNodeSchema, ConsensusDB, DagVoteSchema, NodeSchema, OrderedAnchorIdSchema,
    },
    dag::{CertifiedNode, Node},
};
use aptos_crypto::HashValue;

pub trait DAGStorage: Send + Sync {
    fn save_node(&self, node: &Node) -> anyhow::Result<()>;

    fn delete_node(&self, digest: HashValue) -> anyhow::Result<()>;

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()>;

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>>;

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()>;

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()>;

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>>;

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()>;

    fn save_ordered_anchor_id(&self, node_id: &NodeId) -> anyhow::Result<()>;

    fn get_ordered_anchor_ids(&self) -> anyhow::Result<Vec<(NodeId, ())>>;

    fn delete_ordered_anchor_ids(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()>;
}

impl DAGStorage for ConsensusDB {
    fn save_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.save_data::<NodeSchema>(&node.digest(), node)?)
    }

    fn delete_node(&self, digest: HashValue) -> anyhow::Result<()> {
        Ok(self.delete_data::<NodeSchema>(vec![digest])?)
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        Ok(self.save_data::<DagVoteSchema>(node_id, vote)?)
    }

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        Ok(self.get_all_data::<DagVoteSchema>()?)
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete_data::<DagVoteSchema>(node_ids)?)
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        Ok(self.save_data::<CertifiedNodeSchema>(&node.digest(), node)?)
    }

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        Ok(self.get_all_data::<CertifiedNodeSchema>()?)
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        Ok(self.delete_data::<CertifiedNodeSchema>(digests)?)
    }

    fn save_ordered_anchor_id(&self, node_id: &NodeId) -> anyhow::Result<()> {
        Ok(self.save_data::<OrderedAnchorIdSchema>(node_id, &())?)
    }

    fn get_ordered_anchor_ids(&self) -> anyhow::Result<Vec<(NodeId, ())>> {
        Ok(self.get_all_data::<OrderedAnchorIdSchema>()?)
    }

    fn delete_ordered_anchor_ids(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete_data::<OrderedAnchorIdSchema>(node_ids)?)
    }
}
