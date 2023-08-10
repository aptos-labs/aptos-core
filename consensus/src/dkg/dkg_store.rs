// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::DKGAggNode, DKGNode};
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    validator_verifier::ValidatorVerifier,
};
use dashmap::DashMap;
use tokio::sync::OnceCell;

pub struct DKGStore {
    author: Author,
    // dkg todo: persist the dkg nodes
    // store the mapping from authors to dkg nodes
    nodes: DashMap<Author, DKGNode>,
    // store the partially aggregated transcripts
    agg_trx: Mutex<Option<DKGTranscriptWrapper>>,
    // store the aggregated node containing the final aggregated transcript
    // will be proposed as payload by proposal generator once the OnceCell is set
    agg_node: OnceCell<DKGAggNode>,
}

impl DKGStore {
    pub fn new(author: Author) -> Self {
        Self {
            author,
            nodes: DashMap::new(),
            agg_trx: Mutex::new(None),
            agg_node: OnceCell::new(),
        }
    }

    pub fn add_node(
        &self,
        node: DKGNode,
        validator_verifier: &ValidatorVerifier,
        dkg_pvss_config: &DKGPvssConfig,
    ) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.get().is_some() {
            return Ok(None);
        }
        let author = node.author();
        if self.nodes.contains_key(node.author()) {
            return Err(anyhow::anyhow!(
                "[DKG] Author {:?} sends multiple DKG nodes!",
                author
            ));
        }
        self.nodes.insert(*node.author(), node.clone());

        // Aggregate the transcripts
        let mut agg_trx = self.agg_trx.lock();
        if agg_trx.is_none() {
            *agg_trx = Some(node.transcript().clone());
        } else {
            agg_trx
                .as_mut()
                .unwrap()
                .aggregate_with(dkg_pvss_config, node.transcript());
        }

        let authors: Vec<Author> = self.nodes.iter().map(|entry| *entry.key()).collect();
        // dkg todo: f+1 transcripts are sufficient to reconstruct the aggregated node
        if validator_verifier
            .check_voting_power(authors.iter(), false)
            .is_ok()
        {
            let agg_node = DKGAggNode::new(
                node.epoch(),
                self.author,
                self.agg_trx.lock().take().unwrap(),
            );
            return Ok(Some(agg_node));
        }
        Ok(None)
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.get().is_some() {
            return Ok(None);
        }
        if self.agg_node.set(agg_node.clone()).is_ok() {
            return Ok(Some(agg_node));
        }
        Ok(None)
    }

    pub fn take_agg_node(&mut self) -> Option<DKGAggNode> {
        self.agg_node.take()
    }
}
