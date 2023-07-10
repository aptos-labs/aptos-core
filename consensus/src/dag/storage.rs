// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::NodeDigestSignature, NodeId};
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

    fn save_node_signature(
        &self,
        node_id: &NodeId,
        node_digest_signature: &NodeDigestSignature,
    ) -> anyhow::Result<()>;

    fn get_node_signatures(&self) -> anyhow::Result<HashMap<NodeId, NodeDigestSignature>>;

    fn delete_node_signatures(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()>;

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

    fn save_node_signature(
        &self,
        node_id: &NodeId,
        node_digest_signature: &NodeDigestSignature,
    ) -> anyhow::Result<()> {
        Ok(self.save_node_signature(node_id, node_digest_signature)?)
    }

    fn get_node_signatures(&self) -> anyhow::Result<HashMap<NodeId, NodeDigestSignature>> {
        Ok(self.get_node_signatures()?)
    }

    fn delete_node_signatures(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.delete_node_signatures(node_ids)?)
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
