// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    block_queue::{BlockQueue, BlockQueueItem, RandReadyBlocks},
    types::RandShare,
};
use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    logging::LogEvent,
    pipeline::commit_reliable_broadcast::DropGuard,
    randomness::{rand_manager::log_rand_event, types::ShareAck},
};
use anyhow::bail;
use aptos_consensus_types::common::{Author, Round};
use aptos_dkg::{pvss::Player, weighted_vuf::traits::WeightedVUF};
use aptos_logger::debug;
use aptos_types::randomness::{RandConfig, RandDecision, RandMetadata, Randomness, WVUF};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    time::Duration,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RandItem {
    weight: usize,
    // metadata is set only when block is available
    metadata: Option<RandMetadata>,
    shares: HashMap<Author, RandShare>,
    decision: Option<RandDecision>,
}

impl fmt::Debug for RandItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(weight: {}, decision: {:?})",
            self.weight,
            self.decision.is_some()
        )
    }
}

impl RandItem {
    pub fn new() -> Self {
        Self {
            weight: 0,
            metadata: None,
            shares: HashMap::new(),
            decision: None,
        }
    }

    pub fn shares(&self) -> &HashMap<Author, RandShare> {
        &self.shares
    }

    pub fn decision(&self) -> Option<&RandDecision> {
        self.decision.as_ref()
    }

    pub fn contain_author(&self, author: &Author) -> bool {
        self.shares.contains_key(author)
    }

    pub fn add_share(&mut self, share: RandShare, rand_config: &RandConfig) -> anyhow::Result<()> {
        if let Some(metadata) = self.metadata.as_ref() {
            if metadata != share.metadata() {
                bail!(
                    "[RandStore] RandMetadata mismatch in share {:?}!",
                    share.metadata()
                );
            }
        }

        self.weight += rand_config.get_peer_weight(share.author());
        self.shares.insert(*share.author(), share);
        Ok(())
    }

    pub fn add_decision(&mut self, decision: RandDecision) -> anyhow::Result<()> {
        if let Some(metadata) = self.metadata.as_ref() {
            if metadata != decision.metadata() {
                bail!(
                    "[RandStore] RandMetadata mismatch in decision {:?}!",
                    decision.metadata()
                );
            }
        }
        self.decision = Some(decision);
        Ok(())
    }

    // update metadata and remove inconsistent shares and decision, return decision if available
    pub fn update_metadata(&mut self, metadata: RandMetadata, rand_config: &RandConfig) {
        // assert!(self.metadata.is_none(), "[RandStore] RandMetadata should not be set yet!");
        self.metadata = Some(metadata.clone());
        self.shares.retain(|_, share| *share.metadata() == metadata);
        if self.decision.as_ref().map(|d| d.metadata()) != Some(&metadata) {
            self.decision = None;
        }
        // update weight
        self.weight = 0;
        for share in self.shares.values() {
            self.weight += rand_config.get_peer_weight(share.author());
        }
    }

    pub fn ready_to_aggregate(&self, rand_config: &RandConfig) -> bool {
        self.weight >= rand_config.threshold() && self.metadata.is_some()
    }

    // call aggregate_shares only when all shares have correct metadata
    pub fn aggregate_shares(&mut self, rand_config: &RandConfig) -> anyhow::Result<RandDecision> {
        if let Some(metadata) = &self.metadata {
            let mut apks_and_proofs = vec![];
            for share in self.shares.values() {
                let id = *rand_config
                    .validator
                    .address_to_validator_index()
                    .get(share.author())
                    .unwrap();
                let maybe_apk = rand_config.get_certified_apk(share.author());
                if let Some(apk) = maybe_apk {
                    apks_and_proofs.push((Player { id }, apk.clone(), *share.share()));
                } else {
                    bail!(
                        "[RandStore] No augmented public key for validator id {}, {}",
                        id,
                        share.author()
                    );
                }
            }

            let proof =
                <WVUF as WeightedVUF>::aggregate_shares(&rand_config.wconfig, &apks_and_proofs);
            let eval = <WVUF as WeightedVUF>::derive_eval(
                &rand_config.wconfig,
                &rand_config.vuf_pp,
                metadata.to_bytes().as_slice(),
                rand_config.get_all_certified_apk(),
                &proof,
            )?;
            let eval_bytes = bcs::to_bytes(&eval).unwrap();
            let rand_bytes = Sha3_256::digest(eval_bytes.as_slice()).to_vec();
            let randomness = Randomness::new(metadata.clone(), rand_bytes);
            let decision = RandDecision::new(randomness, eval, proof);

            Ok(decision)
        } else {
            bail!("[RandStore] RandMetadata is None, wait for block to come first!");
        }
    }
}

pub enum AddDecisionResult {
    NewRandReadyBlock,
    None,
}

// RandStore is not required to be persisted
pub struct RandStore {
    pub epoch: u64,
    pub author: Author,
    pub rand_config: Option<RandConfig>,
    pub delta_rb_drop_guard: Option<DropGuard>,
    // all block items
    pub block_queue: BlockQueue,
    // rand todo: persist rand_map
    // all randomness items
    pub rand_map: HashMap<Round, RandItem>,

    // garbage collect rounds < rand_round_min, or > rand_round_max
    pub rand_round_min: Round,
    pub rand_round_max: Round,
    // garbage collect gap from committed_round
    pub gc_gap_below: Round,
    pub gc_gap_above: Round,
}

impl RandStore {
    pub fn new(
        epoch: u64,
        author: Author,
        rand_config: Option<RandConfig>,
        delta_rb_drop_guard: Option<DropGuard>,
        gc_gap_below: Round,
        gc_gap_above: Round,
    ) -> Self {
        Self {
            epoch,
            author,
            rand_config,
            delta_rb_drop_guard,
            block_queue: BlockQueue::new(),
            rand_map: HashMap::new(),
            rand_round_min: 0,
            rand_round_max: Round::max_value(),
            gc_gap_below,
            gc_gap_above,
        }
    }

    pub fn reset(&mut self) {
        self.block_queue = BlockQueue::new();
        // self.rand_map = BTreeMap::new();
        self.rand_round_min = 0;
        self.rand_round_max = Round::max_value();
    }

    pub fn rand_config(&self) -> Option<&RandConfig> {
        self.rand_config.as_ref()
    }

    pub fn block_queue(&self) -> &BTreeMap<Round, BlockQueueItem> {
        self.block_queue.queue()
    }

    pub fn rand_map(&self) -> &HashMap<Round, RandItem> {
        &self.rand_map
    }

    pub fn rebroadcast_rounds(&self, duration: Duration) -> Vec<Round> {
        self.block_queue()
            .iter()
            .flat_map(|(_, item)| {
                item.ordered_blocks
                    .iter()
                    .zip(item.timed_drop_guards.iter())
                    .filter_map(|(block, timed_drop_guard)| {
                        if !block.has_randomness() {
                            if let Some((a, _)) = timed_drop_guard {
                                if a.elapsed() > duration {
                                    Some(block.round())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
            })
            .collect()
    }

    pub fn update_rounds(&mut self, committed_round: Round) {
        self.rand_round_min = committed_round.saturating_sub(self.gc_gap_below);
        self.rand_round_max = committed_round.saturating_add(self.gc_gap_above);
        debug!("[RandStore] updated rounds: committed_round: {}, rand_round_min: {}, rand_round_max: {}", committed_round, self.rand_round_min, self.rand_round_max);
        self.garbage_collect();
    }

    pub fn check_rounds(&self, round: Round) -> anyhow::Result<()> {
        if round < self.rand_round_min || round > self.rand_round_max {
            bail!(
                "[RandStore] round {} is not in range [{}, {}]",
                round,
                self.rand_round_min,
                self.rand_round_max
            );
        }
        Ok(())
    }

    pub fn garbage_collect(&mut self) {
        let mut rounds_to_remove = vec![];
        for round in self.rand_map.keys() {
            if *round < self.rand_round_min || *round > self.rand_round_max {
                rounds_to_remove.push(*round);
            }
        }
        for round in rounds_to_remove {
            self.rand_map.remove(&round);
        }
    }

    pub fn get_my_share(&self, round: &Round) -> Option<&RandShare> {
        self.rand_map
            .get(round)
            .and_then(|rand_item| rand_item.shares().get(&self.author))
    }

    pub fn get_decision(&self, round: &Round) -> Option<&RandDecision> {
        self.rand_map
            .get(round)
            .and_then(|rand_item| rand_item.decision())
    }

    pub fn get_randomness(&self, round: &Round) -> Option<&Randomness> {
        self.get_decision(round)
            .map(|decision| decision.randomness())
    }

    pub fn dequeue_rand_ready_prefix(&mut self) -> Vec<RandReadyBlocks> {
        self.block_queue.dequeue_rand_ready_prefix()
    }

    pub fn add_item(&mut self, item: BlockQueueItem) -> anyhow::Result<Vec<AddDecisionResult>> {
        let metadata_objs: Vec<RandMetadata> = item
            .ordered_blocks
            .iter()
            .map(|b| item.rand_metadata(b.round()))
            .collect();
        self.block_queue.push_back(item);

        let add_decision_results: Vec<AddDecisionResult> = metadata_objs
            .into_iter()
            .map(|metadata| {
                let maybe_decision = self
                    .try_aggregate_shares(metadata.round(), Some(metadata))
                    .unwrap();
                if maybe_decision.is_none() {
                    return AddDecisionResult::None;
                }
                let decision = maybe_decision.unwrap();
                self.add_decision(decision, true).unwrap()
            })
            .collect();

        Ok(add_decision_results)
    }

    pub fn update_guard(&mut self, round: Round, drop_guard: DropGuard) {
        self.block_queue.update_guard(round, drop_guard);
    }

    pub fn try_aggregate_shares(
        &mut self,
        round: Round,
        metadata: Option<RandMetadata>,
    ) -> anyhow::Result<Option<RandDecision>> {
        if self.rand_config.is_none() {
            return Ok(None);
        }
        let rand_config = self.rand_config.as_ref().unwrap();
        let rand_item = self.rand_map.entry(round).or_insert_with(RandItem::new);
        if rand_item.decision().is_some() {
            return Ok(None);
        }
        if let Some(metadata) = metadata {
            rand_item.update_metadata(metadata, rand_config);
        }
        if rand_item.ready_to_aggregate(rand_config) {
            match rand_item.aggregate_shares(rand_config) {
                Ok(decision) => {
                    log_rand_event(
                        LogEvent::AggregateRandDecision,
                        self.author,
                        None,
                        decision.block_id(),
                        decision.round(),
                    );
                    observe_block(decision.timestamp(), BlockStage::RAND_AGG_DECISION);

                    return Ok(Some(decision));
                },
                Err(e) => bail!("{:?}", e),
            }
        }
        Ok(None)
    }

    pub fn add_share(&mut self, share: RandShare) -> anyhow::Result<(ShareAck, AddDecisionResult)> {
        let rand_config = self.rand_config.as_ref().unwrap();
        let missing_apk = rand_config.get_certified_apk(share.author()).is_none();

        if missing_apk {
            bail!("[RandStore] missing apk for {:?}", share.author());
        }

        self.check_rounds(share.round())?;

        let rand_item = self
            .rand_map
            .entry(share.round())
            .or_insert_with(RandItem::new);

        if let Some(decision) = rand_item.decision() {
            return Ok((
                ShareAck::new(self.epoch, Some(decision.clone())),
                AddDecisionResult::None,
            ));
        }

        if rand_item.contain_author(share.author()) {
            if *share.author() == self.author {
                return Ok((ShareAck::new(self.epoch, None), AddDecisionResult::None));
            }
            bail!(
                "[RandStore] duplicate share from the same author {:?}",
                share.author()
            );
        }

        share.verify(rand_config)?;

        rand_item.add_share(share.clone(), rand_config)?;

        observe_block(share.timestamp(), BlockStage::RAND_ADD_SHARE);

        match self.try_aggregate_shares(share.round(), None)? {
            Some(decision) => {
                let ack = ShareAck::new(self.epoch, Some(decision.clone()));
                let add_decision_result = self.add_decision(decision, true)?;
                Ok((ack, add_decision_result))
            },
            None => Ok((ShareAck::new(self.epoch, None), AddDecisionResult::None)),
        }
    }

    pub fn add_decision(
        &mut self,
        decision: RandDecision,
        local: bool,
    ) -> anyhow::Result<AddDecisionResult> {
        let rand_config = self.rand_config.as_ref().unwrap();
        self.check_rounds(decision.round())?;

        let rand_item = self
            .rand_map
            .entry(decision.round())
            .or_insert_with(RandItem::new);

        if rand_item.decision().is_some() {
            return Ok(AddDecisionResult::None);
        }

        if !local {
            decision.verify(rand_config)?;
        }

        rand_item.add_decision(decision.clone())?;

        log_rand_event(
            LogEvent::AddFirstRandDecision,
            self.author,
            None,
            decision.block_id(),
            decision.round(),
        );

        observe_block(decision.timestamp(), BlockStage::RAND_ADD_FIRST_DECISION);

        match self
            .block_queue
            .update_randomness(decision.round(), decision.randomness().clone())
        {
            Err(_e) => Ok(AddDecisionResult::None),
            Ok(()) => Ok(AddDecisionResult::NewRandReadyBlock),
        }
    }
}
