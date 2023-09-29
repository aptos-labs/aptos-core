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
use futures::stream::AbortHandle;

#[derive(Clone)]
pub struct DKGStore {
    author: Author,
    validator_verifier: ValidatorVerifier,
    dkg_pvss_config: DKGPvssConfig,
    start_time: u64,    // None if DKG is not started

    rb_abort_handle: Option<AbortHandle>,

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
}

impl DKGStore {
    pub fn new(author: Author, validator_verifier: ValidatorVerifier, dkg_pvss_config: DKGPvssConfig, start_time: u64) -> Self {
        Self {
            author,
            validator_verifier,
            dkg_pvss_config,
            start_time,
            rb_abort_handle: None,
            nodes: HashMap::new(),
            agg_trx: None,
            agg_node: None,
            agg_node_proposed: false,
        }
    }

    pub fn get_start_time(&self) -> u64 {
        self.start_time
    }

    pub fn get_rb_abort_handle(&self) -> Option<AbortHandle> {
        self.rb_abort_handle.clone()
    }

    pub fn set_rb_abort_handle(&mut self, rb_abort_handle: Option<AbortHandle>) -> Option<AbortHandle> {
        std::mem::replace(&mut self.rb_abort_handle, rb_abort_handle)
    }

    pub fn get_pvss_config(&self) -> &DKGPvssConfig {
        &self.dkg_pvss_config
    }

    pub fn add_node(
        &mut self,
        node: DKGNode,
    ) -> anyhow::Result<Option<DKGAggNode>> {
        if self.agg_node.is_some() {
            // do not add node if the aggregated node is already available
            return Ok(None);
        }

        match node.verify(&self.dkg_pvss_config, &self.validator_verifier) {
            Ok(_) => {
                let author = node.author();
                if self.nodes.contains_key(node.author()) {
                    return Err(anyhow::anyhow!(
                        "[DKG] Author {:?} sends multiple DKG nodes!",
                        author
                    ));
                }

                self.nodes.insert(*node.author(), node.clone());
                debug!("[DKG] Added DKG Node: {:?}, at validator {:?}", node.metadata(), self.author);

                // Aggregate the transcripts
                if self.agg_trx.is_none() {
                    self.agg_trx = Some(node.transcript().clone());
                } else {
                    self.agg_trx.as_mut().unwrap().aggregate_with(&self.dkg_pvss_config, node.transcript());
                }

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
                    if let Err(e) = agg_node.verify(&self.dkg_pvss_config, &self.validator_verifier) {
                        unreachable!("[DKG] Agg trx verify failed: {:?}", e);
                    }
                    debug!(
                        "[DKG] DKGAggNode is ready for epoch {:?} at validator {:?}", node.epoch(), self.author
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

        match agg_node.verify(&self.dkg_pvss_config, &self.validator_verifier)
        {
            Ok(_) => {
                self.agg_node = Some(agg_node.clone());
                debug!("[DKG] Added DKGAggNode: {:?}, at validator {:?}", agg_node.metadata(), self.author);

                return Ok(Some(agg_node));
            }
            Err(e) => {
                anyhow::bail!("[DKG] Failed to verify DKG aggregated node: {:?}, error = {:?}", agg_node.metadata(), e);
            }
        }
    }

    pub fn ready(&self) -> bool {
        self.agg_node.is_some() && !self.agg_node_proposed
    }

    pub fn get_agg_node(&mut self) -> &Option<DKGAggNode> {
        &self.agg_node
    }

    pub fn take_agg_node(&mut self) -> Option<DKGAggNode> {
        self.agg_node_proposed = true;
        self.agg_node.clone()
    }
}
