// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use super::{types::{DKGAggNode, TDKGMessage}, DKGNode};
use aptos_consensus_types::common::Author;
use aptos_logger::debug;
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    validator_verifier::ValidatorVerifier,
};
use tokio::sync::OnceCell;

pub struct DKGStore {
    author: Author,
    validator_verifier: ValidatorVerifier,
    dkg_pvss_config: Option<DKGPvssConfig>,
    // dkg todo: persist the dkg nodes
    // store the mapping from authors to dkg nodes
    nodes: HashMap<Author, DKGNode>,
    // store the partially aggregated transcripts
    agg_trx: Option<DKGTranscriptWrapper>,
    // store the aggregated node containing the final aggregated transcript
    // will be proposed as payload by proposal generator once the OnceCell is set
    agg_node: OnceCell<DKGAggNode>,
    // buffer the nodes received before the DKG locally starts
    buffered_nodes: Vec<DKGNode>,
    buffered_agg_nodes: Vec<DKGAggNode>,
}

impl DKGStore {
    pub fn new(author: Author, validator_verifier: ValidatorVerifier) -> Self {
        Self {
            author,
            validator_verifier,
            dkg_pvss_config: None,
            nodes: HashMap::new(),
            agg_trx: None,
            agg_node: OnceCell::new(),
            buffered_nodes: vec![],
            buffered_agg_nodes: vec![],
        }
    }

    pub fn add_pvss_config(&mut self, dkg_pvss_config: DKGPvssConfig) {
        self.dkg_pvss_config = Some(dkg_pvss_config);
    }

    pub fn get_pvss_config(&self) -> Option<DKGPvssConfig> {
        self.dkg_pvss_config.clone()
    }

    pub fn add_node(
        &mut self,
        node: DKGNode,
    ) -> anyhow::Result<Option<DKGAggNode>> {
        if self.dkg_pvss_config.is_none() {
            self.buffer_nodes(node);
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        } else {
            // dkg todo: need to periodically check if there is any buffered node
            let buffered_nodes = self.take_buffered_nodes();
            for node in buffered_nodes {
                self.add_node(node)?;
            }
        }
        match node.verify(self.dkg_pvss_config.as_ref().unwrap()) {
            Ok(_) => {
                if self.agg_node.get().is_some() {
                    debug!("[DKG] Node {:?} adds DKG Node failed due to agg node already available", self.author);
                    return Ok(None);
                }
                let author = node.author();
                if self.nodes.contains_key(node.author()) {
                    return Err(anyhow::anyhow!(
                        "[DKG] Author {:?} sends multiple DKG nodes!",
                        author
                    ));
                }
                debug!("[DKG] Node {:?} adds DKG Node: {:?}", self.author, node.metadata());

                self.nodes.insert(*node.author(), node.clone());

                // Aggregate the transcripts
                if self.agg_trx.is_none() {
                    self.agg_trx = Some(node.transcript().clone());
                } else {
                    self.agg_trx.as_mut().unwrap().aggregate_with(self.dkg_pvss_config.as_ref().unwrap(), node.transcript());
                }
                debug!("[DKG] Node {:?} aggregates DKG trx: {:?}", self.author, node.metadata());

                let authors: Vec<Author> = self.nodes.iter().map(|(k,_)| *k).collect();

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
                        self.agg_trx.take().unwrap(),
                    );
                    if let Err(e) = agg_node.agg_trx().verify(self.dkg_pvss_config.as_ref().unwrap()) {
                        debug!("[DKG] agg trx verify failed: {:?}", e);
                    }
                    debug!(
                        "[DKG] Node {:?} aggregated transcript is ready for epoch {:?}", self.author,
                        node.epoch()
                    );
                    return Ok(Some(agg_node));
                }
                return Ok(None);
            }
            Err(e) => {
                anyhow::bail!("[DKG] Failed to verify DKG node: {:?}, error = {:?}", node.metadata(), e);
            }
        }
    }

    pub fn add_agg_node(&mut self, agg_node: DKGAggNode) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.get().is_some() {
            return Ok(None);
        }

        if self.dkg_pvss_config.is_none() {
            self.buffer_agg_nodes(agg_node);
            anyhow::bail!("[DKG] DKG PVSS config is not ready!");
        } else {
            // dkg todo: need to periodically check if there is any buffered node
            let buffered_agg_nodes = self.take_buffered_agg_nodes();
            for agg_node in buffered_agg_nodes {
                self.add_agg_node(agg_node)?;
            }
        }
        match agg_node.verify(self.dkg_pvss_config.as_ref().unwrap())
        {
            Ok(_) => {
                if self.agg_node.set(agg_node.clone()).is_ok() {
                    debug!("[DKG] Adding DKG Aggregated Node for epoch {:?}", agg_node.epoch());
                    return Ok(Some(agg_node));
                }
                return Ok(None);
            }
            Err(e) => {
                anyhow::bail!("[DKG] Failed to verify DKG aggregated node: {:?}, error = {:?}", agg_node.metadata(), e);
            }
        }
    }

    pub fn take_agg_node(&mut self) -> Option<DKGAggNode> {
        self.agg_node.take()
    }

    pub fn buffer_nodes(&mut self, node: DKGNode) {
        self.buffered_nodes.push(node);
    }

    pub fn buffer_agg_nodes(&mut self, agg_node: DKGAggNode) {
        self.buffered_agg_nodes.push(agg_node);
    }

    pub fn take_buffered_nodes(&mut self) -> Vec<DKGNode> {
        std::mem::take(&mut self.buffered_nodes)
    }

    pub fn take_buffered_agg_nodes(&mut self) -> Vec<DKGAggNode> {
        std::mem::take(&mut self.buffered_agg_nodes)
    }
}
