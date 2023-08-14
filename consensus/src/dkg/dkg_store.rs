// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{types::{DKGAggNode, TDKGMessage}, DKGNode};
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::debug;
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    validator_verifier::ValidatorVerifier,
};
use dashmap::DashMap;
use tokio::sync::OnceCell;

pub struct DKGStore {
    author: Author,
    validator_verifier: ValidatorVerifier,
    dkg_pvss_config: Mutex<Option<DKGPvssConfig>>,
    // dkg todo: persist the dkg nodes
    // store the mapping from authors to dkg nodes
    nodes: DashMap<Author, DKGNode>,
    // store the partially aggregated transcripts
    agg_trx: Mutex<Option<DKGTranscriptWrapper>>,
    // store the aggregated node containing the final aggregated transcript
    // will be proposed as payload by proposal generator once the OnceCell is set
    agg_node: Mutex<OnceCell<DKGAggNode>>,
    // buffer the nodes received before the DKG locally starts
    buffered_nodes: Mutex<Vec<DKGNode>>,
    buffered_agg_nodes: Mutex<Vec<DKGAggNode>>,
}

impl DKGStore {
    pub fn new(author: Author, validator_verifier: ValidatorVerifier) -> Self {
        Self {
            author,
            validator_verifier,
            dkg_pvss_config: Mutex::new(None),
            nodes: DashMap::new(),
            agg_trx: Mutex::new(None),
            agg_node: Mutex::new(OnceCell::new()),
            buffered_nodes: Mutex::new(vec![]),
            buffered_agg_nodes: Mutex::new(vec![]),
        }
    }

    pub fn add_pvss_config(&self, dkg_pvss_config: DKGPvssConfig) {
        let mut config = self.dkg_pvss_config.lock();
        if config.is_none() {
            *config = Some(dkg_pvss_config);
        }
    }

    pub fn add_node(
        &self,
        node: DKGNode,
    ) -> anyhow::Result<Option<DKGAggNode>> {
        if self.dkg_pvss_config.lock().is_none() {
            self.buffer_nodes(node);
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        } else {
            // dkg todo: need to periodically check if there is any buffered node
            let buffered_nodes = self.take_buffered_nodes();
            for node in buffered_nodes {
                self.add_node(node)?;
            }
        }
        let dkg_pvss_config = self.dkg_pvss_config.lock();
        if node
            .verify(dkg_pvss_config.as_ref().unwrap())
            .is_ok()
        {
            if self.agg_node.lock().get().is_some() {
                debug!("[DKG] Adding DKG Node failed due to agg node already available");
                return Ok(None);
            }
            let author = node.author();
            if self.nodes.contains_key(node.author()) {
                return Err(anyhow::anyhow!(
                    "[DKG] Author {:?} sends multiple DKG nodes!",
                    author
                ));
            }
            debug!("[DKG] Adding DKG Node from author {:?}", author);

            self.nodes.insert(*node.author(), node.clone());

            {
                // Aggregate the transcripts
                let mut agg_trx = self.agg_trx.lock();
                if agg_trx.is_none() {
                    *agg_trx = Some(node.transcript().clone());
                } else {
                    agg_trx
                        .as_mut()
                        .unwrap()
                        .aggregate_with(dkg_pvss_config.as_ref().unwrap(), node.transcript());
                }
                debug!("[DKG] Aggregating DKG trx from author {:?}", author);
            }

            let authors: Vec<Author> = self.nodes.iter().map(|entry| *entry.key()).collect();

            let mut aggregated_voting_power = 0;
            for account_address in authors.clone() {
                match self.validator_verifier.get_voting_power(&account_address) {
                    Some(voting_power) => aggregated_voting_power += voting_power as u128,
                    None => (),
                }
            }
            debug!("[DKG] Node {:?} has aggregated stake {:?}, threshold stake {:?}", self.author, aggregated_voting_power, self.validator_verifier.total_voting_power() - self.validator_verifier.quorum_voting_power());

            // dkg todo: f+1 transcripts are sufficient to reconstruct the aggregated node
            if self.validator_verifier
                .check_voting_power(authors.iter(), false)
                .is_ok()
            {
                let agg_node = DKGAggNode::new(
                    node.epoch(),
                    self.author,
                    self.agg_trx.lock().take().unwrap(),
                );
                debug!(
                    "[DKG] Aggregated transcript is ready for epoch {:?}",
                    node.epoch()
                );
                return Ok(Some(agg_node));
            }
            return Ok(None);
        } else {
            anyhow::bail!("[DKG] Failed to verify DKG node: {:?}", node);
        }
    }

    pub fn add_agg_node(&self, agg_node: DKGAggNode) -> anyhow::Result<Option<DKGAggNode>> {
        debug!("[DKG] adding agg node 2");

        if self.agg_node.lock().get().is_some() {
            return Ok(None);
        }
        debug!("[DKG] adding agg node 3");

        if self.dkg_pvss_config.lock().is_none() {
            debug!("[DKG] adding agg node 0.1");
            self.buffer_agg_nodes(agg_node);
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        } else {
            debug!("[DKG] adding agg node 0.2");
            // dkg todo: need to periodically check if there is any buffered node
            let buffered_agg_nodes = self.take_buffered_agg_nodes();
            for agg_node in buffered_agg_nodes {
                self.add_agg_node(agg_node)?;
            }
        }
        debug!("[DKG] adding agg node 1");
        if agg_node
            .verify(self.dkg_pvss_config.lock().as_ref().unwrap())
            .is_ok()
        {
            if self.agg_node.lock().set(agg_node.clone()).is_ok() {
                debug!("[DKG] Adding DKG Aggregated Node for epoch {:?}", agg_node.epoch());
                return Ok(Some(agg_node));
            }
            return Ok(None);
        } else {
            anyhow::bail!("[DKG] Failed to verify DKG aggregated node: {:?}", agg_node);
        }
    }

    pub fn take_agg_node(&self) -> Option<DKGAggNode> {
        self.agg_node.lock().take()
    }

    pub fn buffer_nodes(&self, node: DKGNode) {
        self.buffered_nodes.lock().push(node);
    }

    pub fn buffer_agg_nodes(&self, agg_node: DKGAggNode) {
        self.buffered_agg_nodes.lock().push(agg_node);
    }

    pub fn take_buffered_nodes(&self) -> Vec<DKGNode> {
        std::mem::take(&mut self.buffered_nodes.lock())
    }

    pub fn take_buffered_agg_nodes(&self) -> Vec<DKGAggNode> {
        std::mem::take(&mut self.buffered_agg_nodes.lock())
    }
}
