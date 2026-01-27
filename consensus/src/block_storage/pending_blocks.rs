// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::block_store::sync_manager::TargetBlockRetrieval,
    counters::BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT,
};
use aptos_consensus_types::{block::Block, common::Round, opt_block_data::OptBlockData};
use aptos_crypto::HashValue;
use aptos_logger::{info, warn};
use aptos_short_hex_str::AsShortHexStr;
use futures_channel::oneshot;
use std::collections::{BTreeMap, HashMap};

/// A local buffer to hold incoming blocks before it reaches round manager.
/// Which can be used to fulfill block request from local due to out of order messages.
pub struct PendingBlocks {
    blocks_by_hash: HashMap<HashValue, Block>,
    blocks_by_round: BTreeMap<Round, Block>,
    opt_blocks_by_round: BTreeMap<Round, OptBlockData>,
    pending_request: Option<(TargetBlockRetrieval, oneshot::Sender<Block>)>,
}

impl PendingBlocks {
    pub fn new() -> Self {
        Self {
            blocks_by_hash: HashMap::new(),
            blocks_by_round: BTreeMap::new(),
            opt_blocks_by_round: BTreeMap::new(),
            pending_request: None,
        }
    }

    pub fn insert_block(&mut self, block: Block) {
        info!("Pending block inserted: {}", block.id());
        self.blocks_by_hash.insert(block.id(), block.clone());
        self.blocks_by_round.insert(block.round(), block.clone());
        if let Some((target_block_retrieval_payload, tx)) = self.pending_request.take() {
            let is_fulfilled = match target_block_retrieval_payload {
                TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                    target_block_id == block.id()
                },
                TargetBlockRetrieval::TargetRound(target_round) => target_round == block.round(),
            };

            if is_fulfilled {
                info!(
                    "FulFill block request from incoming block: {}",
                    target_block_retrieval_payload
                );
                BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                tx.send(block).ok();
            } else {
                self.pending_request = Some((target_block_retrieval_payload, tx));
            }
        }
    }

    pub fn insert_opt_block(&mut self, opt_block_data: OptBlockData) {
        info!(
            "Pending opt block inserted: ({}, {})",
            opt_block_data.author().short_str(),
            opt_block_data.round()
        );
        self.opt_blocks_by_round
            .insert(opt_block_data.round(), opt_block_data.clone());

        let Some(parent_opt_block) = self
            .opt_blocks_by_round
            .remove(&opt_block_data.parent().round())
        else {
            return;
        };

        if parent_opt_block.parent_id() == opt_block_data.grandparent_qc().certified_block().id() {
            let block =
                Block::new_from_opt(parent_opt_block, opt_block_data.grandparent_qc().clone());
            self.insert_block(block);
        } else {
            warn!(
                "Pending Opt Block entry in cache doesn't match QC: {} != {}",
                parent_opt_block, opt_block_data
            );
        }
    }

    pub fn insert_request(
        &mut self,
        target_block_retrieval_payload: TargetBlockRetrieval,
        tx: oneshot::Sender<Block>,
    ) {
        match target_block_retrieval_payload {
            TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                if let Some(block) = self.blocks_by_hash.get(&target_block_id) {
                    info!(
                        "FulFill block request from existing buffer: {}",
                        target_block_id
                    );
                    BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                    tx.send(block.clone()).ok();
                } else {
                    info!("Insert block request for: {}", target_block_id);
                    self.pending_request = Some((target_block_retrieval_payload, tx));
                }
            },
            TargetBlockRetrieval::TargetRound(target_round) => {
                if let Some(block) = self.blocks_by_round.get(&target_round) {
                    info!(
                        "Fulfill block request from existing buffer: {}",
                        target_round
                    );
                    BLOCK_RETRIEVAL_LOCAL_FULFILL_COUNT.inc();
                    tx.send(block.clone()).ok();
                } else {
                    info!("Insert block request for: {}", target_round);
                    self.pending_request = Some((target_block_retrieval_payload, tx));
                }
            },
        }
    }

    pub fn gc(&mut self, round: Round) {
        let mut to_remove = vec![];
        for (r, _) in self.blocks_by_round.range(..=round) {
            to_remove.push(*r);
        }
        for r in to_remove {
            self.opt_blocks_by_round.remove(&r);
            if let Some(block) = self.blocks_by_round.remove(&r) {
                self.blocks_by_hash.remove(&block.id());
            }
        }
    }
}
