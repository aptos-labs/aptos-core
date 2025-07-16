// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensusdb::ConsensusDB, epoch_manager::LivenessStorageData, error::DbError,
    util::calculate_window_start_round,
};
use anyhow::{bail, format_err, Context, Result};
use aptos_config::config::NodeConfig;
use aptos_consensus_types::{
    block::Block, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate, vote::Vote,
    vote_data::VoteData, wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_storage_interface::DbReader;
use aptos_types::{
    block_info::Round, epoch_change::EpochChangeProof, ledger_info::LedgerInfoWithSignatures,
    proof::TransactionAccumulatorSummary, transaction::Version,
};
use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::Arc,
};

/// PersistentLivenessStorage is essential for maintaining liveness when a node crashes.  Specifically,
/// upon a restart, a correct node will recover.  Even if all nodes crash, liveness is
/// guaranteed.
/// Blocks persisted are proposed but not yet committed.  The committed state is persisted
/// via StateComputer.
pub trait PersistentLivenessStorage: Send + Sync {
    /// Persist the blocks and quorum certs into storage atomically.
    fn save_tree(&self, blocks: Vec<Block>, quorum_certs: Vec<QuorumCert>) -> Result<()>;

    /// Delete the corresponding blocks and quorum certs atomically.
    fn prune_tree(&self, block_ids: Vec<HashValue>) -> Result<()>;

    /// Persist consensus' state
    fn save_vote(&self, vote: &Vote) -> Result<()>;

    /// Construct data that can be recovered from ledger
    fn recover_from_ledger(&self) -> LedgerRecoveryData;

    /// Construct necessary data to start consensus.
    fn start(&self, order_vote_enabled: bool, window_size: Option<u64>) -> LivenessStorageData;

    /// Persist the highest 2chain timeout certificate for improved liveness - proof for other replicas
    /// to jump to this round
    fn save_highest_2chain_timeout_cert(
        &self,
        highest_timeout_cert: &TwoChainTimeoutCertificate,
    ) -> Result<()>;

    /// Retrieve a epoch change proof for SafetyRules so it can instantiate its
    /// ValidatorVerifier.
    fn retrieve_epoch_change_proof(&self, version: u64) -> Result<EpochChangeProof>;

    /// Returns a handle of the aptosdb.
    fn aptos_db(&self) -> Arc<dyn DbReader>;

    // Returns a handle of the consensus db
    fn consensus_db(&self) -> Arc<ConsensusDB>;
}

#[derive(Clone)]
pub struct RootInfo {
    pub commit_root_block: Box<Block>,
    /// Genesis `window_root_block` will be None
    pub window_root_block: Option<Box<Block>>,
    pub quorum_cert: QuorumCert,
    pub ordered_cert: WrappedLedgerInfo,
    pub commit_cert: WrappedLedgerInfo,
}

impl Debug for RootInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RootInfo: [commit_root_block: {}, window_root_block: {:?}, quorum_cert: {}, ordered_cert: {}, commit_cert: {}]",
            self.commit_root_block, self.window_root_block, self.quorum_cert, self.ordered_cert, self.commit_cert,
        )
    }
}

/// LedgerRecoveryData is a subset of RecoveryData that we can get solely from ledger info.
#[derive(Clone)]
pub struct LedgerRecoveryData {
    storage_ledger: LedgerInfoWithSignatures,
}

impl LedgerRecoveryData {
    pub fn new(storage_ledger: LedgerInfoWithSignatures) -> Self {
        LedgerRecoveryData { storage_ledger }
    }

    pub fn committed_round(&self) -> Round {
        self.storage_ledger.commit_info().round()
    }

    pub fn find_root_with_window(
        &self,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
        order_vote_enabled: bool,
        window_size: u64,
    ) -> Result<RootInfo> {
        // We start from the block that storage's latest ledger info, if storage has end-epoch
        // LedgerInfo, we generate the virtual genesis block
        let ends_epoch = self.storage_ledger.ledger_info().ends_epoch();
        //TODO(l1-migration): when blocks is empty and we have to create virtual genesis from storage to start consensus.
        // This is only required for migration when we only have a single validator network
        let blocks_empty = blocks.is_empty();
        let (latest_commit_id, latest_ledger_info_sig) = if ends_epoch || blocks_empty {
            let genesis =
                Block::make_genesis_block_from_ledger_info(self.storage_ledger.ledger_info());
            let genesis_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
                self.storage_ledger.ledger_info(),
                genesis.id(),
            );
            let genesis_ledger_info = genesis_qc.ledger_info().clone();
            let genesis_id = genesis.id();
            blocks.push(genesis);
            quorum_certs.push(genesis_qc);
            (genesis_id, genesis_ledger_info)
        } else {
            (
                self.storage_ledger.ledger_info().consensus_block_id(),
                self.storage_ledger.clone(),
            )
        };

        // sort by (epoch, round) to guarantee the topological order of parent <- child
        blocks.sort_by_key(|b| (b.epoch(), b.round()));

        let latest_commit_idx = blocks
            .iter()
            .position(|block| block.id() == latest_commit_id)
            .ok_or_else(|| format_err!("unable to find root: {}", latest_commit_id))?;
        let commit_block = blocks[latest_commit_idx].clone();
        let commit_block_quorum_cert = quorum_certs
            .iter()
            .find(|qc| qc.certified_block().id() == commit_block.id())
            .ok_or_else(|| format_err!("No QC found for root: {}", commit_block.id()))?
            .clone();

        let (root_ordered_cert, root_commit_cert) = if order_vote_enabled {
            // We are setting ordered_root same as commit_root. As every committed block is also ordered, this is fine.
            // As the block store inserts all the fetched blocks and quorum certs and execute the blocks, the block store
            // updates highest_ordered_cert accordingly.
            let root_ordered_cert =
                WrappedLedgerInfo::new(VoteData::dummy(), latest_ledger_info_sig.clone());
            (root_ordered_cert.clone(), root_ordered_cert)
        } else {
            let root_ordered_cert = quorum_certs
                .iter()
                .find(|qc| qc.commit_info().id() == commit_block.id())
                .ok_or_else(|| format_err!("No LI found for root: {}", latest_commit_id))?
                .clone()
                .into_wrapped_ledger_info();
            let root_commit_cert = root_ordered_cert
                .create_merged_with_executed_state(latest_ledger_info_sig)
                .expect("Inconsistent commit proof and evaluation decision, cannot commit block");
            (root_ordered_cert, root_commit_cert)
        };

        let window_start_round = calculate_window_start_round(commit_block.round(), window_size);
        let mut id_to_blocks = HashMap::new();
        blocks.iter().for_each(|block| {
            id_to_blocks.insert(block.id(), block);
        });

        let mut current_block = &commit_block;
        while !current_block.is_genesis_block()
            && current_block.quorum_cert().certified_block().round() >= window_start_round
        {
            if let Some(parent_block) = id_to_blocks.get(&current_block.parent_id()) {
                current_block = *parent_block;
            } else {
                bail!("Parent block not found for block {}", current_block.id());
            }
        }
        let window_start_id = current_block.id();

        let window_start_idx = blocks
            .iter()
            .position(|block| block.id() == window_start_id)
            .ok_or_else(|| format_err!("unable to find window root: {}", window_start_id))?;
        let window_start_block = blocks.remove(window_start_idx);

        info!(
            "Commit block is {}, window block is {}",
            commit_block, window_start_block
        );

        Ok(RootInfo {
            commit_root_block: Box::new(commit_block),
            window_root_block: Some(Box::new(window_start_block)),
            quorum_cert: commit_block_quorum_cert,
            ordered_cert: root_ordered_cert,
            commit_cert: root_commit_cert,
        })
    }

    pub fn find_root_without_window(
        &self,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
        order_vote_enabled: bool,
    ) -> Result<RootInfo> {
        // We start from the block that storage's latest ledger info, if storage has end-epoch
        // LedgerInfo, we generate the virtual genesis block
        let ends_epoch = self.storage_ledger.ledger_info().ends_epoch();
        // TODO(l1-migration): This is for single validator network boostrap. We created virtual genesis to start the consensus
        let blocks_empty = blocks.is_empty();
        let (root_id, latest_ledger_info_sig) = if ends_epoch || blocks_empty {
            let genesis =
                Block::make_genesis_block_from_ledger_info(self.storage_ledger.ledger_info());
            let genesis_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
                self.storage_ledger.ledger_info(),
                genesis.id(),
            );
            let genesis_ledger_info = genesis_qc.ledger_info().clone();
            let genesis_id = genesis.id();
            blocks.push(genesis);
            quorum_certs.push(genesis_qc);
            (genesis_id, genesis_ledger_info)
        } else {
            (
                self.storage_ledger.ledger_info().consensus_block_id(),
                self.storage_ledger.clone(),
            )
        };

        blocks.sort_by_key(|b| (b.epoch(), b.round()));
        let root_idx = blocks
            .iter()
            .position(|block| block.id() == root_id)
            .ok_or_else(|| format_err!("unable to find root: {}", root_id))?;
        let root_block = blocks.remove(root_idx);
        let root_quorum_cert = quorum_certs
            .iter()
            .find(|qc| qc.certified_block().id() == root_block.id())
            .ok_or_else(|| format_err!("No QC found for root: {}", root_id))?
            .clone();

        let (root_ordered_cert, root_commit_cert) = if order_vote_enabled {
            // We are setting ordered_root same as commit_root. As every committed block is also ordered, this is fine.
            // As the block store inserts all the fetched blocks and quorum certs and execute the blocks, the block store
            // updates highest_ordered_cert accordingly.
            let root_ordered_cert =
                WrappedLedgerInfo::new(VoteData::dummy(), latest_ledger_info_sig.clone());
            (root_ordered_cert.clone(), root_ordered_cert)
        } else {
            let root_ordered_cert = quorum_certs
                .iter()
                .find(|qc| qc.commit_info().id() == root_block.id())
                .ok_or_else(|| format_err!("No LI found for root: {}", root_id))?
                .clone()
                .into_wrapped_ledger_info();
            let root_commit_cert = root_ordered_cert
                .create_merged_with_executed_state(latest_ledger_info_sig)
                .expect("Inconsistent commit proof and evaluation decision, cannot commit block");
            (root_ordered_cert, root_commit_cert)
        };
        info!("Consensus root block is {}", root_block);

        Ok(RootInfo {
            commit_root_block: Box::new(root_block),
            window_root_block: None,
            quorum_cert: root_quorum_cert,
            ordered_cert: root_ordered_cert,
            commit_cert: root_commit_cert,
        })
    }

    /// Finds the root (last committed block) and returns the root block, the QC to the root block
    /// and the ledger info for the root block, return an error if it can not be found.
    ///
    /// We guarantee that the block corresponding to the storage's latest ledger info always exists.
    pub fn find_root(
        &self,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
        order_vote_enabled: bool,
        window_size: Option<u64>,
    ) -> Result<RootInfo> {
        info!(
            "The last committed block id as recorded in storage: {}",
            self.storage_ledger
        );

        match window_size {
            None => self.find_root_without_window(blocks, quorum_certs, order_vote_enabled),
            Some(window_size) => {
                self.find_root_with_window(blocks, quorum_certs, order_vote_enabled, window_size)
            },
        }
    }
}

pub struct RootMetadata {
    pub accu_hash: HashValue,
    pub frozen_root_hashes: Vec<HashValue>,
    pub num_leaves: Version,
}

impl RootMetadata {
    pub fn version(&self) -> Version {
        max(self.num_leaves, 1) - 1
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_empty() -> Self {
        Self {
            accu_hash: *aptos_crypto::hash::ACCUMULATOR_PLACEHOLDER_HASH,
            frozen_root_hashes: vec![],
            num_leaves: 0,
        }
    }
}

impl From<TransactionAccumulatorSummary> for RootMetadata {
    fn from(summary: TransactionAccumulatorSummary) -> Self {
        Self {
            accu_hash: summary.0.root_hash,
            frozen_root_hashes: summary.0.frozen_subtree_roots,
            num_leaves: summary.0.num_leaves,
        }
    }
}

/// The recovery data constructed from raw consensusdb data, it'll find the root value and
/// blocks that need cleanup or return error if the input data is inconsistent.
pub struct RecoveryData {
    // The last vote message sent by this validator.
    last_vote: Option<Vote>,
    root: RootInfo,
    root_metadata: RootMetadata,
    // 1. the blocks guarantee the topological ordering - parent <- child.
    // 2. all blocks are children of the root.
    blocks: Vec<Block>,
    quorum_certs: Vec<QuorumCert>,
    blocks_to_prune: Option<Vec<HashValue>>,

    // Liveness data
    highest_2chain_timeout_certificate: Option<TwoChainTimeoutCertificate>,
}

impl RecoveryData {
    pub fn new(
        last_vote: Option<Vote>,
        ledger_recovery_data: LedgerRecoveryData,
        mut blocks: Vec<Block>,
        root_metadata: RootMetadata,
        mut quorum_certs: Vec<QuorumCert>,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
        order_vote_enabled: bool,
        window_size: Option<u64>,
    ) -> Result<Self> {
        let root = ledger_recovery_data
            .find_root(
                &mut blocks,
                &mut quorum_certs,
                order_vote_enabled,
                window_size,
            )
            .with_context(|| {
                // for better readability
                blocks.sort_by_key(|block| block.round());
                quorum_certs.sort_by_key(|qc| qc.certified_block().round());
                format!(
                    "\nRoot: {}\nBlocks in db: {}\nQuorum Certs in db: {}\n",
                    ledger_recovery_data.storage_ledger.ledger_info(),
                    blocks
                        .iter()
                        .map(|b| format!("\n{}", b))
                        .collect::<Vec<String>>()
                        .concat(),
                    quorum_certs
                        .iter()
                        .map(|qc| format!("\n{}", qc))
                        .collect::<Vec<String>>()
                        .concat(),
                )
            })?;

        // If execution pool is enabled, use the window_root, else use the commit_root
        let (root_id, epoch) = match &root.window_root_block {
            None => {
                let commit_root_id = root.commit_root_block.id();
                let epoch = root.commit_root_block.epoch();
                (commit_root_id, epoch)
            },
            Some(window_root_block) => {
                let window_start_id = window_root_block.id();
                let epoch = window_root_block.epoch();
                (window_start_id, epoch)
            },
        };
        let blocks_to_prune = Some(Self::find_blocks_to_prune(
            root_id,
            &mut blocks,
            &mut quorum_certs,
        ));

        Ok(RecoveryData {
            last_vote: match last_vote {
                Some(v) if v.epoch() == epoch => Some(v),
                _ => None,
            },
            root,
            root_metadata,
            blocks,
            quorum_certs,
            blocks_to_prune,
            highest_2chain_timeout_certificate: match highest_2chain_timeout_cert {
                Some(tc) if tc.epoch() == epoch => Some(tc),
                _ => None,
            },
        })
    }

    pub fn commit_root_block(&self) -> &Block {
        &self.root.commit_root_block
    }

    pub fn last_vote(&self) -> Option<Vote> {
        self.last_vote.clone()
    }

    pub fn take(self) -> (RootInfo, RootMetadata, Vec<Block>, Vec<QuorumCert>) {
        (
            self.root,
            self.root_metadata,
            self.blocks,
            self.quorum_certs,
        )
    }

    pub fn take_blocks_to_prune(&mut self) -> Vec<HashValue> {
        self.blocks_to_prune
            .take()
            .expect("blocks_to_prune already taken")
    }

    pub fn highest_2chain_timeout_certificate(&self) -> Option<TwoChainTimeoutCertificate> {
        self.highest_2chain_timeout_certificate.clone()
    }

    fn find_blocks_to_prune(
        root_id: HashValue,
        blocks: &mut Vec<Block>,
        quorum_certs: &mut Vec<QuorumCert>,
    ) -> Vec<HashValue> {
        // prune all the blocks that don't have root as ancestor
        let mut tree = HashSet::new();
        let mut to_remove = HashSet::new();
        tree.insert(root_id);
        // assume blocks are sorted by round already
        blocks.retain(|block| {
            if tree.contains(&block.parent_id()) {
                tree.insert(block.id());
                true
            } else {
                to_remove.insert(block.id());
                false
            }
        });
        quorum_certs.retain(|qc| {
            if tree.contains(&qc.certified_block().id()) {
                true
            } else {
                to_remove.insert(qc.certified_block().id());
                false
            }
        });
        to_remove.into_iter().collect()
    }
}

/// The proxy we use to persist data in db storage service via grpc.
pub struct StorageWriteProxy {
    db: Arc<ConsensusDB>,
    aptos_db: Arc<dyn DbReader>,
}

impl StorageWriteProxy {
    pub fn new(config: &NodeConfig, aptos_db: Arc<dyn DbReader>) -> Self {
        let db = Arc::new(ConsensusDB::new(config.storage.dir()));
        StorageWriteProxy { db, aptos_db }
    }
}

impl PersistentLivenessStorage for StorageWriteProxy {
    fn save_tree(&self, blocks: Vec<Block>, quorum_certs: Vec<QuorumCert>) -> Result<()> {
        Ok(self
            .db
            .save_blocks_and_quorum_certificates(blocks, quorum_certs)?)
    }

    fn prune_tree(&self, block_ids: Vec<HashValue>) -> Result<()> {
        if !block_ids.is_empty() {
            // quorum certs that certified the block_ids will get removed
            self.db.delete_blocks_and_quorum_certificates(block_ids)?;
        }
        Ok(())
    }

    fn save_vote(&self, vote: &Vote) -> Result<()> {
        Ok(self.db.save_vote(bcs::to_bytes(vote)?)?)
    }

    fn recover_from_ledger(&self) -> LedgerRecoveryData {
        let latest_ledger_info = self
            .aptos_db
            .get_latest_ledger_info()
            .expect("Failed to get latest ledger info.");
        LedgerRecoveryData::new(latest_ledger_info)
    }

    fn start(&self, order_vote_enabled: bool, window_size: Option<u64>) -> LivenessStorageData {
        info!("Start consensus recovery.");
        let raw_data = self
            .db
            .get_data()
            .expect("unable to recover consensus data");

        let last_vote = raw_data
            .0
            .map(|bytes| bcs::from_bytes(&bytes[..]).expect("unable to deserialize last vote"));

        let highest_2chain_timeout_cert = raw_data.1.map(|b| {
            bcs::from_bytes(&b).expect("unable to deserialize highest 2-chain timeout cert")
        });
        let blocks = raw_data.2;
        let quorum_certs: Vec<_> = raw_data.3;
        let blocks_repr: Vec<String> = blocks.iter().map(|b| format!("\n\t{}", b)).collect();
        info!(
            "The following blocks were restored from ConsensusDB : {}",
            blocks_repr.concat()
        );
        let qc_repr: Vec<String> = quorum_certs
            .iter()
            .map(|qc| format!("\n\t{}", qc))
            .collect();
        info!(
            "The following quorum certs were restored from ConsensusDB: {}",
            qc_repr.concat()
        );
        // find the block corresponding to storage latest ledger info
        let latest_ledger_info = self
            .aptos_db
            .get_latest_ledger_info()
            .expect("Failed to get latest ledger info.");
        let accumulator_summary = self
            .aptos_db
            .get_accumulator_summary(latest_ledger_info.ledger_info().version())
            .expect("Failed to get accumulator summary.");
        let ledger_recovery_data = LedgerRecoveryData::new(latest_ledger_info);

        match RecoveryData::new(
            last_vote,
            ledger_recovery_data.clone(),
            blocks,
            accumulator_summary.into(),
            quorum_certs,
            highest_2chain_timeout_cert,
            order_vote_enabled,
            window_size,
        ) {
            Ok(mut initial_data) => {
                (self as &dyn PersistentLivenessStorage)
                    .prune_tree(initial_data.take_blocks_to_prune())
                    .expect("unable to prune dangling blocks during restart");
                if initial_data.last_vote.is_none() {
                    self.db
                        .delete_last_vote_msg()
                        .expect("unable to cleanup last vote");
                }
                if initial_data.highest_2chain_timeout_certificate.is_none() {
                    self.db
                        .delete_highest_2chain_timeout_certificate()
                        .expect("unable to cleanup highest 2-chain timeout cert");
                }
                info!(
                    "Starting up the consensus state machine with recovery data - [last_vote {}], [highest timeout certificate: {}]",
                    initial_data.last_vote.as_ref().map_or_else(|| "None".to_string(), |v| v.to_string()),
                    initial_data.highest_2chain_timeout_certificate().as_ref().map_or_else(|| "None".to_string(), |v| v.to_string()),
                );

                LivenessStorageData::FullRecoveryData(initial_data)
            },
            Err(e) => {
                error!(error = ?e, "Failed to construct recovery data");
                LivenessStorageData::PartialRecoveryData(ledger_recovery_data)
            },
        }
    }

    fn save_highest_2chain_timeout_cert(
        &self,
        highest_timeout_cert: &TwoChainTimeoutCertificate,
    ) -> Result<()> {
        Ok(self
            .db
            .save_highest_2chain_timeout_certificate(bcs::to_bytes(highest_timeout_cert)?)?)
    }

    fn retrieve_epoch_change_proof(&self, version: u64) -> Result<EpochChangeProof> {
        let (_, proofs) = self
            .aptos_db
            .get_state_proof(version)
            .map_err(DbError::from)?
            .into_inner();
        Ok(proofs)
    }

    fn aptos_db(&self) -> Arc<dyn DbReader> {
        self.aptos_db.clone()
    }

    fn consensus_db(&self) -> Arc<ConsensusDB> {
        self.db.clone()
    }
}
