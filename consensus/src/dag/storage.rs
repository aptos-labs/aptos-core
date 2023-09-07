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
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()>;

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>>;

    fn delete_pending_node(&self) -> anyhow::Result<()>;

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
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.put::<NodeSchema>(&(), node)?)
    }

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>> {
        Ok(self.get::<NodeSchema>(&())?)
    }

    fn delete_pending_node(&self) -> anyhow::Result<()> {
        Ok(self.delete::<NodeSchema>(vec![()])?)
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        Ok(self.put::<DagVoteSchema>(node_id, vote)?)
    }

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        Ok(self.get_all::<DagVoteSchema>()?)
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete::<DagVoteSchema>(node_ids)?)
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        Ok(self.put::<CertifiedNodeSchema>(&node.digest(), node)?)
    }

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        Ok(self.get_all::<CertifiedNodeSchema>()?)
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        Ok(self.delete::<CertifiedNodeSchema>(digests)?)
    }

    fn save_ordered_anchor_id(&self, node_id: &NodeId) -> anyhow::Result<()> {
        Ok(self.put::<OrderedAnchorIdSchema>(node_id, &())?)
    }

    fn get_ordered_anchor_ids(&self) -> anyhow::Result<Vec<(NodeId, ())>> {
        Ok(self.get_all::<OrderedAnchorIdSchema>()?)
    }

    fn delete_ordered_anchor_ids(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete::<OrderedAnchorIdSchema>(node_ids)?)
    }
}
