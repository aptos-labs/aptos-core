// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

#[cfg(test)]
mod test;

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::APTOS_EXECUTOR_OTHER_TIMERS_SECONDS,
};
use anyhow::{anyhow, ensure, Result};
use aptos_consensus_types::block::Block as ConsensusBlock;
use aptos_crypto::HashValue;
use aptos_executor_types::{Error, ExecutedBlock};
use aptos_infallible::Mutex;
use aptos_logger::{debug, info};
use aptos_storage_interface::DbReader;
use aptos_types::{ledger_info::LedgerInfo, proof::definition::LeafCount};
use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{
        mpsc::{channel, Receiver},
        Arc, Weak,
    },
};

pub struct Block {
    pub id: HashValue,
    pub output: ExecutedBlock,
    children: Mutex<Vec<Arc<Block>>>,
    block_lookup: Arc<BlockLookup>,
}

impl Drop for Block {
    fn drop(&mut self) {
        self.block_lookup.remove(self.id);
        debug!(
            LogSchema::new(LogEntry::SpeculationCache).block_id(self.id),
            "Block dropped."
        );
    }
}

impl Block {
    fn add_child(&self, child: Arc<Self>) {
        self.children.lock().push(child)
    }

    pub fn num_persisted_transactions(&self) -> LeafCount {
        self.output.result_view.txn_accumulator().num_leaves()
    }

    pub fn ensure_has_child(&self, child_id: HashValue) -> Result<()> {
        ensure!(
            self.children.lock().iter().any(|c| c.id == child_id),
            "{:x} doesn't have child {:x}",
            self.id,
            child_id,
        );
        Ok(())
    }
}

fn epoch_genesis_block_id(ledger_info: &LedgerInfo) -> HashValue {
    ConsensusBlock::make_genesis_block_from_ledger_info(ledger_info).id()
}

struct BlockLookupInner(HashMap<HashValue, Weak<Block>>);

impl BlockLookupInner {
    fn multi_get(&self, ids: &[HashValue]) -> Result<Vec<Option<Arc<Block>>>> {
        let mut blocks = Vec::with_capacity(ids.len());
        for id in ids {
            let block = self
                .0
                .get(id)
                .map(|weak| {
                    weak.upgrade()
                        .ok_or_else(|| anyhow!("Block {:x} has been deallocated.", id))
                })
                .transpose()?;
            blocks.push(block)
        }
        Ok(blocks)
    }

    fn get(&self, id: HashValue) -> Result<Option<Arc<Block>>> {
        Ok(self.multi_get(&[id])?.pop().expect("Must exist."))
    }

    fn fetch_or_add_block(
        &mut self,
        id: HashValue,
        output: ExecutedBlock,
        parent_id: Option<HashValue>,
        block_lookup: &Arc<BlockLookup>,
    ) -> Result<(Arc<Block>, bool, Option<Arc<Block>>)> {
        let parent_block = parent_id
            .map(|id| {
                self.get(id)?
                    .ok_or_else(|| anyhow!("parent block {:x} doesn't exist.", id))
            })
            .transpose()?;

        match self.0.entry(id) {
            Entry::Occupied(entry) => {
                let existing = entry
                    .get()
                    .upgrade()
                    .ok_or_else(|| anyhow!("block dropped unexpected."))?;
                ensure!(
                    existing
                        .output
                        .result_view
                        .is_same_view(&output.result_view),
                    "Different block with same id {:x}",
                    id,
                );
                Ok((existing, true, parent_block))
            },
            Entry::Vacant(entry) => {
                let block = Arc::new(Block {
                    id,
                    output,
                    children: Mutex::new(Vec::new()),
                    block_lookup: block_lookup.clone(),
                });
                entry.insert(Arc::downgrade(&block));
                Ok((block, false, parent_block))
            },
        }
    }
}

struct BlockLookup {
    inner: Mutex<BlockLookupInner>,
}

impl BlockLookup {
    fn new() -> Self {
        Self {
            inner: Mutex::new(BlockLookupInner(HashMap::new())),
        }
    }

    fn multi_get(&self, ids: &[HashValue]) -> Result<Vec<Option<Arc<Block>>>> {
        self.inner.lock().multi_get(ids)
    }

    fn fetch_or_add_block(
        self: &Arc<Self>,
        id: HashValue,
        output: ExecutedBlock,
        parent_id: Option<HashValue>,
    ) -> Result<Arc<Block>> {
        let (block, existing, parent_block) = self
            .inner
            .lock()
            .fetch_or_add_block(id, output, parent_id, self)?;

        if let Some(parent_block) = parent_block {
            if existing {
                parent_block.ensure_has_child(id)?;
            } else {
                parent_block.add_child(block.clone());
            }
        }

        Ok(block)
    }

    fn remove(&self, id: HashValue) {
        self.inner.lock().0.remove(&id);
    }
}

pub struct BlockTree {
    root: Mutex<Arc<Block>>,
    block_lookup: Arc<BlockLookup>,
}

impl BlockTree {
    pub fn new(db: &Arc<dyn DbReader>) -> Result<Self> {
        let block_lookup = Arc::new(BlockLookup::new());
        let root = Mutex::new(Self::root_from_db(&block_lookup, db)?);

        Ok(Self { root, block_lookup })
    }

    pub fn reset(&self, db: &Arc<dyn DbReader>) -> Result<()> {
        *self.root.lock() = Self::root_from_db(&self.block_lookup, db)?;
        Ok(())
    }

    pub fn get_block(&self, id: HashValue) -> Result<Arc<Block>> {
        Ok(self.get_blocks(&[id])?.pop().expect("Must exist."))
    }

    pub fn get_blocks(&self, ids: &[HashValue]) -> Result<Vec<Arc<Block>>> {
        let lookup_result = self.block_lookup.multi_get(ids)?;

        itertools::zip_eq(ids, lookup_result)
            .map(|(id, res)| res.ok_or_else(|| Error::BlockNotFound(*id).into()))
            .collect()
    }

    pub fn get_blocks_opt(&self, ids: &[HashValue]) -> Result<Vec<Option<Arc<Block>>>> {
        self.block_lookup.multi_get(ids)
    }

    fn root_from_db(block_lookup: &Arc<BlockLookup>, db: &Arc<dyn DbReader>) -> Result<Arc<Block>> {
        let ledger_info_with_sigs = db.get_latest_ledger_info()?;
        let ledger_info = ledger_info_with_sigs.ledger_info();
        let ledger_view = db.get_latest_executed_trees()?;

        ensure!(
            ledger_view.version() == Some(ledger_info.version()),
            "Missing ledger info at the end of the ledger. latest version {:?}, LI version {}",
            ledger_view.version(),
            ledger_info.version(),
        );

        let id = if ledger_info.ends_epoch() {
            epoch_genesis_block_id(ledger_info)
        } else {
            ledger_info.consensus_block_id()
        };

        block_lookup.fetch_or_add_block(id, ExecutedBlock::new_empty(ledger_view), None)
    }

    // Set the root to be at `ledger_info`, drop blocks that are no longer descendants of the
    // new root.
    //
    // Dropping happens asynchronously in another thread. A receiver is returned to the caller
    // to wait for the dropping to fully complete (useful for tests).
    pub fn prune(&self, ledger_info: &LedgerInfo) -> Result<Receiver<()>> {
        let committed_block_id = ledger_info.consensus_block_id();
        let last_committed_block = self.get_block(committed_block_id)?;

        let root = if ledger_info.ends_epoch() {
            let epoch_genesis_id = epoch_genesis_block_id(ledger_info);
            info!(
                LogSchema::new(LogEntry::SpeculationCache)
                    .root_block_id(epoch_genesis_id)
                    .original_reconfiguration_block_id(committed_block_id),
                "Updated with a new root block as a virtual block of reconfiguration block"
            );
            self.block_lookup.fetch_or_add_block(
                epoch_genesis_id,
                ExecutedBlock::new_empty(last_committed_block.output.result_view.clone()),
                None,
            )?
        } else {
            info!(
                LogSchema::new(LogEntry::SpeculationCache).root_block_id(committed_block_id),
                "Updated with a new root block",
            );
            last_committed_block
        };
        let old_root = {
            let mut root_locked = self.root.lock();
            // send old root to async task to drop it
            let old_root = root_locked.clone();
            *root_locked = root;
            old_root
        };
        // This should be the last reference to old root, spawning a drop to a different thread
        // guarantees that the drop will not happen in the current thread
        let (tx, rx) = channel::<()>();
        rayon::spawn(move || {
            let _timeer = APTOS_EXECUTOR_OTHER_TIMERS_SECONDS
                .with_label_values(&["drop_old_root"])
                .start_timer();
            drop(old_root);
            // Error is ignored, since the caller might not care about dropping completion and
            // has discarded the receiver already.
            tx.send(()).ok();
        });
        Ok(rx)
    }

    pub fn add_block(
        &self,
        parent_block_id: HashValue,
        id: HashValue,
        output: ExecutedBlock,
    ) -> Result<Arc<Block>> {
        self.block_lookup
            .fetch_or_add_block(id, output, Some(parent_block_id))
    }

    pub fn root_block(&self) -> Arc<Block> {
        self.root.lock().clone()
    }
}
