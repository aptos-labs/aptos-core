// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::common::Author;
use aptos_types::validator_verifier::ValidatorVerifier;
use dashmap::DashMap;
use tokio::sync::OnceCell;

use super::{DKGNode, types::{DKGAggNode, TDKGMessage}};

#[derive(Clone)]
pub struct DKGStore {
    // dkg todo: persist the dkg nodes
    // stores the mapping from authors to dkg nodes
    nodes: DashMap<Author, DKGNode>,
    agg_node: OnceCell<DKGAggNode>,
}

impl DKGStore {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            agg_node: OnceCell::new(),
        }
    }

    pub fn add_node(&self, node: DKGNode, validator_verifier: &ValidatorVerifier) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.get().is_some() {
            return Ok(None);
        }
        let author = node.author();
        if self.nodes.contains_key(node.author()) {
            return Err(anyhow::anyhow!("[DKG] Author {:?} sends multiple DKG nodes!", author));
        }
        if node.verify(validator_verifier).is_ok() {
            self.nodes.insert(*node.author(), node);
            let authors: Vec<Author> = self.nodes.iter().map(|entry| entry.key().clone()).collect();
            // f+1 transcripts are sufficient to reconstruct the aggregated node
            if validator_verifier.check_voting_power(authors.iter(), false).is_ok() {
                // dkg todo: aggregate the transcripts and produced aggregated node
                let node = self.nodes.iter().next().unwrap().clone();
                let agg_node = DKGAggNode::new(node.epoch(), *node.author(), node.transcript().clone());
                return Ok(Some(agg_node));
            }
            return Ok(None);
        }
        return Err(anyhow::anyhow!("[DKG] Author {:?} sends invalid DKG node!\n node: {:?} \n", node.author(), node));
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode, validator_verifier: &ValidatorVerifier) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.get().is_some() {
            return Ok(None);
        }
        if agg_node.verify(validator_verifier).is_ok() {
            if self.agg_node.set(agg_node.clone()).is_ok() {
                // Broadcast the first aggregated dkg node
                return Ok(Some(agg_node));
            }
            return Ok(None);
        } else {
            return Err(anyhow::anyhow!("[DKG] Author {:?} sends invalid aggregated DKG node!\n node: {:?} \n", agg_node.author(), agg_node));
        }
    }
}
