// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensusdb::ConsensusDB,
    dag::{CertifiedNode, Node},
};
use aptos_crypto::HashValue;
use std::collections::HashMap;

pub trait DAGStorage {
    fn save_node(&self, node: &Node) -> anyhow::Result<()>;

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()>;

    fn get_certified_nodes(&self) -> anyhow::Result<HashMap<HashValue, CertifiedNode>>;

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()>;
}

impl DAGStorage for ConsensusDB {
    fn save_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.save_node(node)?)
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
