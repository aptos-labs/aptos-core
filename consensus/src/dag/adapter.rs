// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dag_store::DagStore,
    // observability::counters::{NUM_NODES_PER_BLOCK, NUM_ROUNDS_PER_BLOCK},
};
use crate::{
    // block_storage::tracing::{BlockStage},
    consensusdb::{CertifiedNodeSchema, ConsensusDB, DagVoteSchema, NodeSchema},
    // counters::update_counters_for_committed_blocks,
    dag::{
        storage::{CommitEvent, DAGStorage},
        CertifiedNode, Node, NodeId, Vote,
    },
    // pipeline::buffer_manager::OrderedBlocks,
};
use anyhow::{anyhow, bail};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Round},
    quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
// use aptos_executor_types::StateComputeResult;
use aptos_infallible::RwLock;
use aptos_logger::{error, info};
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_config::NewBlockEvent,
    // aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfoWithSignatures},
};
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, Instant},
};

pub trait OrderedNotifier: Send + Sync {
    fn send_ordered_nodes(
        &self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    );
}

#[async_trait]
pub trait ProofNotifier: Send + Sync {
    async fn send_epoch_change(&self, proof: EpochChangeProof);

    async fn send_commit_proof(&self, ledger_info: LedgerInfoWithSignatures);
}

pub(crate) fn compute_initial_block_and_ledger_info(
    ledger_info_from_storage: LedgerInfoWithSignatures,
) -> (BlockInfo, LedgerInfoWithSignatures) {
    // We start from the block that storage's latest ledger info, if storage has end-epoch
    // LedgerInfo, we generate the virtual genesis block
    if ledger_info_from_storage.ledger_info().ends_epoch() {
        let genesis =
            Block::make_genesis_block_from_ledger_info(ledger_info_from_storage.ledger_info());

        let ledger_info = ledger_info_from_storage.ledger_info();
        let genesis_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
            ledger_info_from_storage.ledger_info(),
            genesis.id(),
        );
        let genesis_ledger_info = genesis_qc.ledger_info().clone();
        (
            genesis.gen_block_info(
                ledger_info.transaction_accumulator_hash(),
                ledger_info.version(),
                ledger_info.next_epoch_state().cloned(),
            ),
            genesis_ledger_info,
        )
    } else {
        (
            ledger_info_from_storage.ledger_info().commit_info().clone(),
            ledger_info_from_storage,
        )
    }
}

pub(super) struct OrderedNotifierAdapter {
    dag_id: u8,
    executor_channel: tokio::sync::mpsc::UnboundedSender<ShoalppOrderBlocksInfo>,
    dag: Arc<DagStore>,
    epoch_state: Arc<EpochState>,
    ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
    block_ordered_ts: Arc<RwLock<BTreeMap<Round, Instant>>>,
}

impl OrderedNotifierAdapter {
    pub(super) fn new(
        dag_id: u8,
        executor_channel: tokio::sync::mpsc::UnboundedSender<ShoalppOrderBlocksInfo>,
        dag: Arc<DagStore>,
        epoch_state: Arc<EpochState>,
        ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
    ) -> Self {
        Self {
            dag_id,
            executor_channel,
            dag,
            epoch_state,
            ledger_info_provider,
            block_ordered_ts: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub(super) fn pipeline_pending_latency(&self) -> Duration {
        match self.block_ordered_ts.read().first_key_value() {
            Some((round, timestamp)) => {
                let latency = timestamp.elapsed();
                info!(round = round, latency = latency, "pipeline pending latency");
                latency
            },
            None => Duration::ZERO,
        }
    }
}

pub struct ShoalppOrderBlocksInfo {
    pub dag_id: u8,
    pub ordered_nodes: Vec<Arc<CertifiedNode>>,
    pub failed_author: Vec<(Round, Author)>,
}

impl ShoalppOrderBlocksInfo {
    pub fn new(dag_id: u8,
               ordered_nodes: Vec<Arc<CertifiedNode>>,
               failed_author: Vec<(Round, Author)>,
    ) -> Self {
        Self {
            dag_id,
            ordered_nodes,
            failed_author,
        }
    }
}

impl OrderedNotifier for OrderedNotifierAdapter {
    fn send_ordered_nodes(
        &self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) {
        let block_info = ShoalppOrderBlocksInfo::new(self.dag_id, ordered_nodes, failed_author);
        if self
            .executor_channel
            .send(block_info)
            .is_err()
        {
            error!("[DAG] execution pipeline closed");
        }
    }
}

pub struct StorageAdapter {
    epoch: u64,
    epoch_to_validators: HashMap<u64, Vec<Author>>,
    consensus_db: Arc<ConsensusDB>,
    aptos_db: Arc<dyn DbReader>,
}

impl StorageAdapter {
    pub fn new(
        epoch: u64,
        epoch_to_validators: HashMap<u64, Vec<Author>>,
        consensus_db: Arc<ConsensusDB>,
        aptos_db: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            epoch,
            epoch_to_validators,
            consensus_db,
            aptos_db,
        }
    }

    pub fn bitvec_to_validators(
        validators: &[Author],
        bitvec: &BitVec,
    ) -> anyhow::Result<Vec<Author>> {
        if BitVec::required_buckets(validators.len() as u16) != bitvec.num_buckets() {
            bail!(
                "bitvec bucket {} does not match validators len {}",
                bitvec.num_buckets(),
                validators.len()
            );
        }

        Ok(validators
            .iter()
            .enumerate()
            .filter_map(|(index, validator)| {
                if bitvec.is_set(index as u16) {
                    Some(*validator)
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn indices_to_validators(
        validators: &[Author],
        indices: &[u64],
    ) -> anyhow::Result<Vec<Author>> {
        indices
            .iter()
            .map(|index| {
                usize::try_from(*index)
                    .map_err(|_err| anyhow!("index {} out of bounds", index))
                    .and_then(|index| {
                        validators.get(index).cloned().ok_or(anyhow!(
                            "index {} is larger than number of validators {}",
                            index,
                            validators.len()
                        ))
                    })
            })
            .collect()
    }

    fn convert(&self, new_block_event: NewBlockEvent) -> anyhow::Result<CommitEvent> {
        let validators = &self.epoch_to_validators[&new_block_event.epoch()];
        Ok(CommitEvent::new(
            NodeId::new(
                new_block_event.epoch(),
                new_block_event.round(),
                new_block_event.proposer(),
            ),
            Self::bitvec_to_validators(
                validators,
                &new_block_event.previous_block_votes_bitvec().clone().into(),
            )?,
            Self::indices_to_validators(validators, new_block_event.failed_proposer_indices())?,
        ))
    }
}

impl DAGStorage for StorageAdapter {
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()> {
        Ok(self.consensus_db.put::<NodeSchema>(&(), node)?)
    }

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>> {
        Ok(self.consensus_db.get::<NodeSchema>(&())?)
    }

    fn delete_pending_node(&self) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<NodeSchema>(vec![()])?)
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        Ok(self.consensus_db.put::<DagVoteSchema>(node_id, vote)?)
    }

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        Ok(self.consensus_db.get_all::<DagVoteSchema>()?)
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<DagVoteSchema>(node_ids)?)
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        Ok(self
            .consensus_db
            .put::<CertifiedNodeSchema>(&node.digest(), node)?)
    }

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        Ok(self.consensus_db.get_all::<CertifiedNodeSchema>()?)
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        Ok(self.consensus_db.delete::<CertifiedNodeSchema>(digests)?)
    }

    fn get_latest_k_committed_events(&self, k: u64) -> anyhow::Result<Vec<CommitEvent>> {
        let mut commit_events = vec![];
        for event in self.aptos_db.get_latest_block_events(k as usize)? {
            let new_block_event = bcs::from_bytes::<NewBlockEvent>(event.event.event_data())?;
            if self
                .epoch_to_validators
                .contains_key(&new_block_event.epoch())
            {
                commit_events.push(self.convert(new_block_event)?);
            }
        }
        commit_events.reverse();
        Ok(commit_events)
    }

    fn get_latest_ledger_info(&self) -> anyhow::Result<LedgerInfoWithSignatures> {
        // TODO: use callback from notifier to cache the latest ledger info
        Ok(self.aptos_db.get_latest_ledger_info()?)
    }
}

pub(crate) trait TLedgerInfoProvider: Send + Sync {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures;

    fn get_highest_committed_anchor_round(&self, dag_id: u8) -> Round;
}

pub struct LedgerInfoProvider {
    latest_ledger_info: LedgerInfoWithSignatures,
}

impl LedgerInfoProvider {
    pub(super) fn new(mut latest_ledger_info: LedgerInfoWithSignatures, new_epoch: u64) -> Self {
        let epoch = latest_ledger_info.ledger_info().epoch();
        if new_epoch > epoch {
            // TODO: verify it does what I think it does.
            let committed_rounds = HashValue::new([0; HashValue::LENGTH]);
            latest_ledger_info.set_consensus_data_hash(committed_rounds);
        }
        Self { latest_ledger_info }
    }

    pub(super) fn notify_commit_proof(&mut self, ledger_info: LedgerInfoWithSignatures) {
        self.latest_ledger_info = ledger_info;
    }
}

impl TLedgerInfoProvider for RwLock<LedgerInfoProvider> {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures {
        self.read().latest_ledger_info.clone()
    }

    fn get_highest_committed_anchor_round(&self, dag_id: u8) -> Round {
        let committed_anchor_rounds = self
            .read()
            .latest_ledger_info
            .get_highest_committed_rounds_for_bolt();
        committed_anchor_rounds[dag_id as usize]
    }
}
