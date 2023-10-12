// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{collections::{HashMap, BTreeMap}, time::Duration, fmt};

use anyhow::bail;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::debug;
use aptos_types::randomness::{Randomness, RandConfig, RandDecision, RandProof};

use crate::{logging::LogEvent, block_storage::tracing::{BlockStage, observe_block}, randomness::rand_manager::log_rand_event, experimental::commit_reliable_broadcast::DropGuard};

use super::{block_queue::{BlockQueue, BlockQueueItem, RandReadyBlocks}, types::RandShare};

pub struct RandItem {
    weight: u64,
    shares: HashMap<Author, RandShare>,
    decision: Option<RandDecision>,
}

impl fmt::Debug for RandItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(weight: {}, decision: {:?})", self.weight, self.decision.is_some())
    }
}

impl RandItem {
    pub fn new() -> Self {
        Self {
            weight: 0,
            shares: HashMap::new(),
            decision: None,
        }
    }

    pub fn weight(&self) -> u64 {
        self.weight
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

    pub fn add_share(&mut self, share: RandShare) {
        self.weight += share.weight();
        self.shares.insert(*share.author(), share);
    }

    pub fn add_decision(&mut self, decision: RandDecision) {
        self.decision = Some(decision);
    }
}

pub enum AddDecisionResult {
    NewRandReadyBlock,
    None,
}

// RandStore is not required to be persisted
pub struct RandStore {
    author: Author,
    rand_config: RandConfig,
    // all block items
    block_queue: BlockQueue,
    // rand todo: persist rand_map
    // all randomness items
    rand_map: HashMap<Round, RandItem>,

    // garbage collect rounds < rand_round_min, or > rand_round_max
    rand_round_min: Round,
    rand_round_max: Round,
    // garbage collect gap from committed_round
    gc_gap_below: Round,
    gc_gap_above: Round,
}

impl RandStore {
    pub fn new(author: Author, rand_config: RandConfig, gc_gap_below: Round, gc_gap_above: Round) -> Self {
        Self {
            author,
            rand_config,
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

    pub fn rand_config(&self) -> &RandConfig {
        &self.rand_config
    }

    pub fn block_queue(&self) -> &BTreeMap<Round, BlockQueueItem> {
        &self.block_queue.queue()
    }

    pub fn rand_map(&self) -> &HashMap<Round, RandItem> {
        &self.rand_map
    }

    pub fn rebroadcast_rounds(&self, duration: Duration) -> Vec<Round> {
        self
            .block_queue()
            .iter()
            .filter_map(|(round, item)| {
                if let BlockQueueItem::Ordered(ordered) = item {
                    ordered
                        .timed_drop_guard
                        .as_ref()
                        .filter(|(start_time, _)| start_time.elapsed() > duration)
                        .map(|_| *round)
                } else {
                    None
                }
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
            bail!("[RandStore] round {:?} is not in range [{:?}, {:?}]", round, self.rand_round_min, self.rand_round_max);
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
        self.rand_map.get(round).and_then(|rand_item| rand_item.shares().get(&self.author))
    }

    pub fn get_decision(&self, round: &Round) -> Option<&RandDecision> {
        self.rand_map.get(round).and_then(|rand_item| rand_item.decision())
    }

    pub fn get_randomness(&self, round: &Round) -> Option<&Randomness> {
        self.get_decision(round).map(|decision| decision.randomness())
    }

    pub fn dequeue_rand_ready_prefix(&mut self) -> Vec<RandReadyBlocks> {
        self.block_queue.dequeue_rand_ready_prefix()
    }

    pub fn add_block(&mut self, block: BlockQueueItem) {
        self.block_queue.push_back(block);
    }

    pub fn update_guard(&mut self, round: Round, drop_guard: DropGuard) {
        self.block_queue.update_guard(round, drop_guard);
    }

    pub fn add_share(&mut self, share: RandShare) -> anyhow::Result<(Option<RandDecision>, AddDecisionResult)> {
        self.check_rounds(share.round())?;

        let rand_item = self.rand_map.entry(share.round()).or_insert_with(RandItem::new);

        if let Some(decision) = rand_item.decision() {
            return Ok((Some(decision.clone()), AddDecisionResult::None));
        }

        if rand_item.contain_author(share.author()) {
            if *share.author() == self.author {
                return Ok((None, AddDecisionResult::None));
            }
            bail!("[RandStore] duplicate share from the same author {:?}", share.author());
        }

        share.verify(&self.rand_config)?;

        rand_item.add_share(share.clone());

        observe_block(share.block_info().timestamp_usecs(), BlockStage::RAND_ADD_SHARE);

        debug!("[RandStore] Added share for round {:?} from {:?}, weight {} threshold {}", share.round(), share.author(), rand_item.weight(), self.rand_config.weight_f());

        if rand_item.weight() >= self.rand_config.weight_f() {
            log_rand_event(LogEvent::AggregateRandDecision, self.author, Some(*share.author()), share.id(), share.round());

            observe_block(share.block_info().timestamp_usecs(), BlockStage::RAND_AGG_DECISION);

            // rand todo: aggregate the shares
            // rand todo: generate real decision

            let dummy_decision = Randomness::new_for_test(share.epoch(), share.round(), share.id(), share.timestamp());
            let decision = RandDecision::new(dummy_decision, RandProof::new_for_test());

            let result = self.add_decision(decision.clone())?;

            return Ok((Some(decision), result));
        }
        Ok((None, AddDecisionResult::None))
    }

    pub fn add_decision(&mut self, decision: RandDecision) -> anyhow::Result<AddDecisionResult> {
        self.check_rounds(decision.round())?;

        let rand_item = self.rand_map.entry(decision.round()).or_insert_with(RandItem::new);
        
        if rand_item.decision().is_some() {
            return Ok(AddDecisionResult::None);
        }

        decision.verify(&self.rand_config)?;

        rand_item.add_decision(decision.clone());

        debug!("[RandStore] Added decision for round {:?}", decision.round());

        log_rand_event(LogEvent::AddFirstRandDecision, self.author, None, decision.block_id(), decision.round());

        observe_block(decision.timestamp(), BlockStage::RAND_ADD_FIRST_DECISION);

        match self.block_queue.update_randomness(decision.round(), decision.randomness().clone()) {
            Err(e) => {
                debug!("{:?}", e);
                Ok(AddDecisionResult::None)
            }
            Ok(()) => {
                debug!("[RandStore] Updated block with decision for round {:?}", decision.round());
                Ok(AddDecisionResult::NewRandReadyBlock)
            }
        }
    }
}