// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    counters::update_counters_for_committed_blocks,
    logging::{LogEvent, LogSchema},
    persistent_liveness_storage::PersistentLivenessStorage,
};
use anyhow::bail;
use aptos_consensus_types::{
    pipelined_block::PipelinedBlock, quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeoutCertificate, wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    block_info::{BlockInfo, Round},
    ledger_info::LedgerInfoWithSignatures,
};
use mirai_annotations::precondition;
use std::{
    collections::{vec_deque::VecDeque, BTreeMap, HashMap, HashSet},
    sync::Arc,
};

/// This structure is a wrapper of [`ExecutedBlock`](aptos_consensus_types::pipelined_block::PipelinedBlock)
/// that adds `children` field to know the parent-child relationship between blocks.
struct LinkableBlock {
    /// Executed block that has raw block data and execution output.
    executed_block: Arc<PipelinedBlock>,
    /// The set of children for cascading pruning. Note: a block may have multiple children.
    children: HashSet<HashValue>,
}

impl LinkableBlock {
    pub fn new(block: PipelinedBlock) -> Self {
        Self {
            executed_block: Arc::new(block),
            children: HashSet::new(),
        }
    }

    pub fn executed_block(&self) -> &Arc<PipelinedBlock> {
        &self.executed_block
    }

    pub fn children(&self) -> &HashSet<HashValue> {
        &self.children
    }

    pub fn add_child(&mut self, child_id: HashValue) {
        assert!(
            self.children.insert(child_id),
            "Block {:x} already existed.",
            child_id,
        );
    }
}

impl LinkableBlock {
    pub fn id(&self) -> HashValue {
        self.executed_block().id()
    }
}

/// This structure maintains a consistent block tree of parent and children links. Blocks contain
/// parent links and are immutable.  For all parent links, a child link exists. This structure
/// should only be used internally in BlockStore.
pub struct BlockTree {
    /// All the blocks known to this replica (with parent links)
    id_to_block: HashMap<HashValue, LinkableBlock>,
    /// Root of the tree. This is the root of ordering phase
    ordered_root_id: HashValue,
    /// Commit Root id: this is the root of commit phase
    commit_root_id: HashValue,
    /// A certified block id with highest round
    highest_certified_block_id: HashValue,

    /// The quorum certificate of highest_certified_block
    highest_quorum_cert: Arc<QuorumCert>,
    /// The highest 2-chain timeout certificate (if any).
    highest_2chain_timeout_cert: Option<Arc<TwoChainTimeoutCertificate>>,
    /// The quorum certificate that has highest commit info.
    highest_ordered_cert: Arc<WrappedLedgerInfo>,
    /// The quorum certificate that has highest commit decision info.
    highest_commit_cert: Arc<WrappedLedgerInfo>,
    /// Map of block id to its completed quorum certificate (2f + 1 votes)
    id_to_quorum_cert: HashMap<HashValue, Arc<QuorumCert>>,
    /// To keep the IDs of the elements that have been pruned from the tree but not cleaned up yet.
    pruned_block_ids: VecDeque<HashValue>,
    /// Num pruned blocks to keep in memory.
    max_pruned_blocks_in_mem: usize,

    /// Round to Block index. We expect only one block per round.
    round_to_ids: BTreeMap<Round, HashValue>,
}

impl BlockTree {
    pub(super) fn new(
        root: PipelinedBlock,
        root_quorum_cert: QuorumCert,
        root_ordered_cert: WrappedLedgerInfo,
        root_commit_cert: WrappedLedgerInfo,
        max_pruned_blocks_in_mem: usize,
        highest_2chain_timeout_cert: Option<Arc<TwoChainTimeoutCertificate>>,
    ) -> Self {
        assert_eq!(
            root.id(),
            root_ordered_cert.commit_info().id(),
            "inconsistent root and ledger info"
        );
        let root_id = root.id();

        let mut id_to_block = HashMap::new();
        let mut round_to_ids = BTreeMap::new();
        round_to_ids.insert(root.round(), root_id);
        id_to_block.insert(root_id, LinkableBlock::new(root));
        counters::NUM_BLOCKS_IN_TREE.set(1);

        let root_quorum_cert = Arc::new(root_quorum_cert);
        let mut id_to_quorum_cert = HashMap::new();
        id_to_quorum_cert.insert(
            root_quorum_cert.certified_block().id(),
            Arc::clone(&root_quorum_cert),
        );

        let pruned_block_ids = VecDeque::with_capacity(max_pruned_blocks_in_mem);

        BlockTree {
            id_to_block,
            ordered_root_id: root_id,
            commit_root_id: root_id, // initially we set commit_root_id = root_id
            highest_certified_block_id: root_id,
            highest_quorum_cert: Arc::clone(&root_quorum_cert),
            highest_ordered_cert: Arc::new(root_ordered_cert),
            highest_commit_cert: Arc::new(root_commit_cert),
            id_to_quorum_cert,
            pruned_block_ids,
            max_pruned_blocks_in_mem,
            highest_2chain_timeout_cert,
            round_to_ids,
        }
    }

    // This method will only be used in this module.
    fn get_linkable_block(&self, block_id: &HashValue) -> Option<&LinkableBlock> {
        self.id_to_block.get(block_id)
    }

    // This method will only be used in this module.
    fn get_linkable_block_mut(&mut self, block_id: &HashValue) -> Option<&mut LinkableBlock> {
        self.id_to_block.get_mut(block_id)
    }

    /// fetch all the quorum certs with non-empty commit info
    pub fn get_all_quorum_certs_with_commit_info(&self) -> Vec<QuorumCert> {
        return self
            .id_to_quorum_cert
            .values()
            .filter(|qc| qc.commit_info() != &BlockInfo::empty())
            .map(|qc| (**qc).clone())
            .collect::<Vec<QuorumCert>>();
    }

    // This method will only be used in this module.
    // This method is used in pruning and length query,
    // to reflect the actual root, we use commit root
    fn linkable_root(&self) -> &LinkableBlock {
        self.get_linkable_block(&self.commit_root_id)
            .expect("Root must exist")
    }

    fn remove_block(&mut self, block_id: HashValue) {
        // Remove the block from the store
        if let Some(block) = self.id_to_block.remove(&block_id) {
            let round = block.executed_block().round();
            self.round_to_ids.remove(&round);
        };
        self.id_to_quorum_cert.remove(&block_id);
    }

    pub(super) fn block_exists(&self, block_id: &HashValue) -> bool {
        self.id_to_block.contains_key(block_id)
    }

    pub(super) fn get_block(&self, block_id: &HashValue) -> Option<Arc<PipelinedBlock>> {
        self.get_linkable_block(block_id)
            .map(|lb| lb.executed_block().clone())
    }

    pub(super) fn get_block_for_round(&self, round: Round) -> Option<Arc<PipelinedBlock>> {
        self.round_to_ids
            .get(&round)
            .and_then(|block_id| self.get_block(block_id))
    }

    pub(super) fn ordered_root(&self) -> Arc<PipelinedBlock> {
        self.get_block(&self.ordered_root_id)
            .expect("Root must exist")
    }

    pub(super) fn commit_root(&self) -> Arc<PipelinedBlock> {
        self.get_block(&self.commit_root_id)
            .expect("Commit root must exist")
    }

    pub(super) fn highest_certified_block(&self) -> Arc<PipelinedBlock> {
        self.get_block(&self.highest_certified_block_id)
            .expect("Highest cerfified block must exist")
    }

    pub(super) fn highest_quorum_cert(&self) -> Arc<QuorumCert> {
        Arc::clone(&self.highest_quorum_cert)
    }

    pub(super) fn highest_2chain_timeout_cert(&self) -> Option<Arc<TwoChainTimeoutCertificate>> {
        self.highest_2chain_timeout_cert.clone()
    }

    /// Replace highest timeout cert with the given value.
    pub(super) fn replace_2chain_timeout_cert(&mut self, tc: Arc<TwoChainTimeoutCertificate>) {
        self.highest_2chain_timeout_cert.replace(tc);
    }

    pub(super) fn highest_ordered_cert(&self) -> Arc<WrappedLedgerInfo> {
        Arc::clone(&self.highest_ordered_cert)
    }

    pub(super) fn highest_commit_cert(&self) -> Arc<WrappedLedgerInfo> {
        Arc::clone(&self.highest_commit_cert)
    }

    pub(super) fn get_quorum_cert_for_block(
        &self,
        block_id: &HashValue,
    ) -> Option<Arc<QuorumCert>> {
        self.id_to_quorum_cert.get(block_id).cloned()
    }

    pub(super) fn insert_block(
        &mut self,
        block: PipelinedBlock,
    ) -> anyhow::Result<Arc<PipelinedBlock>> {
        let block_id = block.id();
        if let Some(existing_block) = self.get_block(&block_id) {
            debug!("Already had block {:?} for id {:?} when trying to add another block {:?} for the same id",
                       existing_block,
                       block_id,
                       block);
            Ok(existing_block)
        } else {
            match self.get_linkable_block_mut(&block.parent_id()) {
                Some(parent_block) => parent_block.add_child(block_id),
                None => bail!("Parent block {} not found", block.parent_id()),
            };
            let linkable_block = LinkableBlock::new(block);
            let arc_block = Arc::clone(linkable_block.executed_block());
            assert!(self.id_to_block.insert(block_id, linkable_block).is_none());
            // Note: the assumption is that we have/enforce unequivocal proposer election.
            if let Some(old_block_id) = self.round_to_ids.get(&arc_block.round()) {
                warn!(
                    "Multiple blocks received for round {}. Previous block id: {}",
                    arc_block.round(),
                    old_block_id
                );
            } else {
                self.round_to_ids.insert(arc_block.round(), block_id);
            }
            counters::NUM_BLOCKS_IN_TREE.inc();
            Ok(arc_block)
        }
    }

    fn update_highest_commit_cert(&mut self, new_commit_cert: WrappedLedgerInfo) {
        if new_commit_cert.commit_info().round() > self.highest_commit_cert.commit_info().round() {
            self.highest_commit_cert = Arc::new(new_commit_cert);
            self.update_commit_root(self.highest_commit_cert.commit_info().id());
        }
    }

    pub(super) fn insert_quorum_cert(&mut self, qc: QuorumCert) -> anyhow::Result<()> {
        let block_id = qc.certified_block().id();
        let qc = Arc::new(qc);

        // Safety invariant: For any two quorum certificates qc1, qc2 in the block store,
        // qc1 == qc2 || qc1.round != qc2.round
        // The invariant is quadratic but can be maintained in linear time by the check
        // below.
        precondition!({
            let qc_round = qc.certified_block().round();
            self.id_to_quorum_cert.values().all(|x| {
                (*(*x).ledger_info()).ledger_info().consensus_data_hash()
                    == (*(*qc).ledger_info()).ledger_info().consensus_data_hash()
                    || x.certified_block().round() != qc_round
            })
        });

        match self.get_block(&block_id) {
            Some(block) => {
                if block.round() > self.highest_certified_block().round() {
                    self.highest_certified_block_id = block.id();
                    self.highest_quorum_cert = Arc::clone(&qc);
                }
            },
            None => bail!("Block {} not found", block_id),
        }

        self.id_to_quorum_cert
            .entry(block_id)
            .or_insert_with(|| Arc::clone(&qc));

        if self.highest_ordered_cert.commit_info().round() < qc.commit_info().round() {
            // Question: We are updating highest_ordered_cert but not highest_ordered_root. Is that fine?
            self.highest_ordered_cert = Arc::new(qc.into_wrapped_ledger_info());
        }

        Ok(())
    }

    pub fn insert_ordered_cert(&mut self, ordered_cert: WrappedLedgerInfo) {
        if ordered_cert.commit_info().round() > self.highest_ordered_cert.commit_info().round() {
            self.highest_ordered_cert = Arc::new(ordered_cert);
        }
    }

    /// Find the blocks to prune up to next_root_id (keep next_root_id's block). Any branches not
    /// part of the next_root_id's tree should be removed as well.
    ///
    /// For example, root = B0
    /// B0--> B1--> B2
    ///        ╰--> B3--> B4
    ///
    /// prune_tree(B_3) should be left with
    /// B3--> B4, root = B3
    ///
    /// Note this function is read-only, use with process_pruned_blocks to do the actual prune.
    pub(super) fn find_blocks_to_prune(&self, next_root_id: HashValue) -> VecDeque<HashValue> {
        // Nothing to do if this is the commit root
        if next_root_id == self.commit_root_id {
            return VecDeque::new();
        }

        let mut blocks_pruned = VecDeque::new();
        let mut blocks_to_be_pruned = vec![self.linkable_root()];
        while let Some(block_to_remove) = blocks_to_be_pruned.pop() {
            block_to_remove.executed_block().abort_pipeline();
            // Add the children to the blocks to be pruned (if any), but stop when it reaches the
            // new root
            for child_id in block_to_remove.children() {
                if next_root_id == *child_id {
                    continue;
                }
                blocks_to_be_pruned.push(
                    self.get_linkable_block(child_id)
                        .expect("Child must exist in the tree"),
                );
            }
            // Track all the block ids removed
            blocks_pruned.push_back(block_to_remove.id());
        }
        blocks_pruned
    }

    pub(super) fn update_ordered_root(&mut self, root_id: HashValue) {
        assert!(self.block_exists(&root_id));
        self.ordered_root_id = root_id;
    }

    pub(super) fn update_commit_root(&mut self, root_id: HashValue) {
        assert!(self.block_exists(&root_id));
        self.commit_root_id = root_id;
    }

    /// Process the data returned by the prune_tree, they're separated because caller might
    /// be interested in doing extra work e.g. delete from persistent storage.
    /// Note that we do not necessarily remove the pruned blocks: they're kept in a separate buffer
    /// for some time in order to enable other peers to retrieve the blocks even after they've
    /// been committed.
    pub(super) fn process_pruned_blocks(&mut self, mut newly_pruned_blocks: VecDeque<HashValue>) {
        counters::NUM_BLOCKS_IN_TREE.sub(newly_pruned_blocks.len() as i64);
        // The newly pruned blocks are pushed back to the deque pruned_block_ids.
        // In case the overall number of the elements is greater than the predefined threshold,
        // the oldest elements (in the front of the deque) are removed from the tree.
        self.pruned_block_ids.append(&mut newly_pruned_blocks);
        if self.pruned_block_ids.len() > self.max_pruned_blocks_in_mem {
            let num_blocks_to_remove = self.pruned_block_ids.len() - self.max_pruned_blocks_in_mem;
            for _ in 0..num_blocks_to_remove {
                if let Some(id) = self.pruned_block_ids.pop_front() {
                    self.remove_block(id);
                }
            }
        }
    }

    /// Returns all the blocks between the commit root and the given block, including the given block
    /// but excluding the root.
    /// In case a given block is not the successor of the root, return None.
    /// While generally the provided blocks should always belong to the active tree, there might be
    /// a race, in which the root of the tree is propagated forward between retrieving the block
    /// and getting its path from root (e.g., at proposal generator). Hence, we don't want to panic
    /// and prefer to return None instead.
    pub(super) fn path_from_root_to_block(
        &self,
        block_id: HashValue,
        root_id: HashValue,
        root_round: u64,
    ) -> Option<Vec<Arc<PipelinedBlock>>> {
        let mut res = vec![];
        let mut cur_block_id = block_id;
        loop {
            match self.get_block(&cur_block_id) {
                Some(ref block) if block.round() <= root_round => {
                    break;
                },
                Some(block) => {
                    cur_block_id = block.parent_id();
                    res.push(block);
                },
                None => return None,
            }
        }
        // At this point cur_block.round() <= self.root.round()
        if cur_block_id != root_id {
            return None;
        }
        // Called `.reverse()` to get the chronically increased order.
        res.reverse();
        Some(res)
    }

    pub(super) fn path_from_ordered_root(
        &self,
        block_id: HashValue,
    ) -> Option<Vec<Arc<PipelinedBlock>>> {
        self.path_from_root_to_block(block_id, self.ordered_root_id, self.ordered_root().round())
    }

    pub(super) fn path_from_commit_root(
        &self,
        block_id: HashValue,
    ) -> Option<Vec<Arc<PipelinedBlock>>> {
        self.path_from_root_to_block(block_id, self.commit_root_id, self.commit_root().round())
    }

    pub(super) fn max_pruned_blocks_in_mem(&self) -> usize {
        self.max_pruned_blocks_in_mem
    }

    /// Update the counters for committed blocks and prune them from the in-memory and persisted store.
    pub fn commit_callback_deprecated(
        &mut self,
        storage: Arc<dyn PersistentLivenessStorage>,
        blocks_to_commit: &[Arc<PipelinedBlock>],
        finality_proof: WrappedLedgerInfo,
        commit_decision: LedgerInfoWithSignatures,
    ) {
        let commit_proof = finality_proof
            .create_merged_with_executed_state(commit_decision)
            .expect("Inconsistent commit proof and evaluation decision, cannot commit block");
        update_counters_for_committed_blocks(blocks_to_commit);
        let last_block = blocks_to_commit.last().expect("pipeline is empty").clone();

        let block_id = last_block.id();
        let block_round = last_block.round();

        self.commit_callback(storage, block_id, block_round, commit_proof);
    }

    pub fn commit_callback(
        &mut self,
        storage: Arc<dyn PersistentLivenessStorage>,
        block_id: HashValue,
        block_round: Round,
        commit_proof: WrappedLedgerInfo,
    ) {
        let current_round = self.commit_root().round();
        let committed_round = block_round;

        debug!(
            LogSchema::new(LogEvent::CommitViaBlock).round(current_round),
            committed_round = committed_round,
            block_id = block_id,
        );

        let ids_to_remove = self.find_blocks_to_prune(block_id);
        if let Err(e) = storage.prune_tree(ids_to_remove.clone().into_iter().collect()) {
            // it's fine to fail here, as long as the commit succeeds, the next restart will clean
            // up dangling blocks, and we need to prune the tree to keep the root consistent with
            // executor.
            warn!(error = ?e, "fail to delete block");
        }
        self.process_pruned_blocks(ids_to_remove);
        self.update_highest_commit_cert(commit_proof);
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl BlockTree {
    /// Returns the number of blocks in the tree
    pub(super) fn len(&self) -> usize {
        // BFS over the tree to find the number of blocks in the tree.
        let mut res = 0;
        let mut to_visit = vec![self.linkable_root()];
        while let Some(block) = to_visit.pop() {
            res += 1;
            for child_id in block.children() {
                to_visit.push(
                    self.get_linkable_block(child_id)
                        .expect("Child must exist in the tree"),
                );
            }
        }
        res
    }

    /// Returns the number of child links in the tree
    pub(super) fn child_links(&self) -> usize {
        self.len() - 1
    }

    /// The number of pruned blocks that are still available in memory
    pub(super) fn pruned_blocks_in_mem(&self) -> usize {
        self.pruned_block_ids.len()
    }
}
