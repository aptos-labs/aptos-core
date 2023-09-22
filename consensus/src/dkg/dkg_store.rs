// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use super::{types::{DKGAggNode, TDKGMessage}, DKGNode};
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, error};
use aptos_types::{
    dkg::{DKGPvssConfig, DKGTranscriptWrapper},
    validator_verifier::ValidatorVerifier,
};

#[derive(Clone)]
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
    // will be proposed as payload by proposal generator
    agg_node: Option<DKGAggNode>,
    // true if the aggregated node is already pulled by the proposal generator
    agg_node_proposed: bool,
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
            agg_node: None,
            agg_node_proposed: false,
            buffered_nodes: vec![],
            buffered_agg_nodes: vec![],
        }
    }

    pub fn add_pvss_config(&mut self, dkg_pvss_config: DKGPvssConfig) {
        self.dkg_pvss_config = Some(dkg_pvss_config);
        // add buffered nodes received before the DKG locally starts
        let buffered_nodes = self.take_buffered_nodes();
        for node in buffered_nodes {
            if let Err(e) = self.add_node(node) {
                error!("[DKG] Error when adding DKG node: {:?}", e);
            }
        }
        let buffered_agg_nodes = self.take_buffered_agg_nodes();
        for agg_node in buffered_agg_nodes {
            if let Err(e) = self.add_agg_node(agg_node) {
                error!("[DKG] Error when adding DKG aggregated node: {:?}", e);
            }
        }
    }

    pub fn get_pvss_config(&self) -> Option<DKGPvssConfig> {
        self.dkg_pvss_config.clone()
    }

    pub fn add_node(
        &mut self,
        node: DKGNode,
    ) -> anyhow::Result<Option<DKGAggNode>> {
        if self.dkg_pvss_config.is_none() {
            debug!("[DKG] Node {:?} pvss config is not ready! receiving node {:?}", self.author, node.metadata());
            self.buffer_nodes(node);
            return Ok(None);
        }

        if self.agg_node.is_some() {
            // do not add node if the aggregated node is already available
            return Ok(None);
        }

        match node.verify(self.dkg_pvss_config.as_ref().unwrap(), &self.validator_verifier) {
            Ok(_) => {
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

                // transcripts from > one third stakes are sufficient to reconstruct the aggregated node
                if self.validator_verifier
                    .check_voting_power(authors.iter(), false)
                    .is_ok()
                {
                    let agg_node = DKGAggNode::new(
                        node.epoch(),
                        self.author,
                        self.agg_trx.clone().unwrap(),
                    );
                    if let Err(e) = agg_node.verify(self.dkg_pvss_config.as_ref().unwrap(), &self.validator_verifier) {
                        unreachable!("[DKG] agg trx verify failed: {:?}", e);
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
        if self.agg_node.is_some() {
            return Ok(None);
        }

        if self.dkg_pvss_config.is_none() {
            debug!("[DKG] Node {:?} DKG PVSS config is not ready! receiving agg node {:?}", self.author, agg_node.metadata());
            self.buffer_agg_nodes(agg_node);
            return Ok(None);
        }
        match agg_node.verify(self.dkg_pvss_config.as_ref().unwrap(), &self.validator_verifier)
        {
            Ok(_) => {
                self.agg_node = Some(agg_node.clone());
                debug!("[DKG] Adding DKG Aggregated Node for epoch {:?}", agg_node.epoch());
                return Ok(Some(agg_node));
            }
            Err(e) => {
                anyhow::bail!("[DKG] Failed to verify DKG aggregated node: {:?}, error = {:?}", agg_node.metadata(), e);
            }
        }
    }

    pub fn ready(&self) -> bool {
        self.agg_node.is_some() && self.get_pvss_config().is_some() && !self.agg_node_proposed
    }

    pub fn get_agg_node(&mut self) -> &Option<DKGAggNode> {
        &self.agg_node
    }

    pub fn take_agg_node(&mut self) -> Option<DKGAggNode> {
        self.agg_node_proposed = true;
        self.agg_node.clone()
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
