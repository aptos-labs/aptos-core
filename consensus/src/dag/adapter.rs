// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dag_store::DagStore,
    // observability::counters::{NUM_NODES_PER_BLOCK, NUM_ROUNDS_PER_BLOCK},
};
use crate::{
    consensusdb::{
        ConsensusDB, Dag0CertifiedNodeSchema, Dag0NodeSchema, Dag0VoteSchema,
        Dag1CertifiedNodeSchema, Dag1NodeSchema, Dag1VoteSchema, Dag2CertifiedNodeSchema,
        Dag2NodeSchema, Dag2VoteSchema, RocksdbPropertyReporter,
    },
    counters,
    dag::{
        observability::counters::BLOCK_COUNTER,
        storage::{CommitEvent, DAGStorage},
        CertifiedNode, Node, NodeId, Vote,
    },
};
use anyhow::{anyhow, bail, format_err};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Round},
    quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::{debug, error, info};
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_config::NewBlockEvent,
    // aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    on_chain_config::CommitHistoryResource,
    state_store::state_key::StateKey,
};
use async_trait::async_trait;
use std::{
    collections::{BTreeMap, HashMap},
    mem,
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
        let mut genesis_ledger_info = genesis_qc.ledger_info().clone();
        let committed_rounds = HashValue::new([0; HashValue::LENGTH]);
        genesis_ledger_info.set_consensus_data_hash(committed_rounds);
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

struct RoundIndexGenerator {
    current_round: Round,
    current_idx: u64,
}

impl RoundIndexGenerator {
    fn next(&mut self, anchor_round: Round) -> Round {
        assert!(self.current_round <= anchor_round);
        if anchor_round == self.current_round {
            self.current_idx += 1;
        } else {
            self.current_idx = 0;
            self.current_round = anchor_round;
        }
        self.current_idx
    }
}

pub(super) struct OrderedNotifierAdapter {
    dag_id: u8,
    executor_channel: tokio::sync::mpsc::UnboundedSender<ShoalppOrderBlocksInfo>,
    dag: Arc<DagStore>,
    epoch_state: Arc<EpochState>,
    ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
    block_ordered_ts: Arc<RwLock<BTreeMap<Round, Instant>>>,
    buffer: Mutex<Vec<Arc<CertifiedNode>>>,
    allow_batches_without_pos_in_proposal: bool,
    author_to_index: HashMap<Author, usize>,
    idx_gen: Mutex<RoundIndexGenerator>,
}

impl OrderedNotifierAdapter {
    pub(super) fn new(
        dag_id: u8,
        executor_channel: tokio::sync::mpsc::UnboundedSender<ShoalppOrderBlocksInfo>,
        dag: Arc<DagStore>,
        epoch_state: Arc<EpochState>,
        ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
        allow_batches_without_pos_in_proposal: bool,
    ) -> Self {
        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let shoalpp_dag_round = ledger_info_provider
            .read()
            .get_highest_committed_shoalpp_round(dag_id);
        let current_round = shoalpp_dag_round / epoch_state.verifier.len() as u64;
        let current_idx = shoalpp_dag_round % epoch_state.verifier.len() as u64;
        Self {
            dag_id,
            executor_channel,
            dag,
            epoch_state,
            ledger_info_provider,
            block_ordered_ts: Arc::new(RwLock::new(BTreeMap::new())),
            allow_batches_without_pos_in_proposal,
            author_to_index,
            idx_gen: Mutex::new(RoundIndexGenerator {
                current_round,
                current_idx,
            }),
            buffer: Mutex::new(Vec::with_capacity(150)),
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
    pub idx: u64,
    pub dag_external_round: Round,
}

impl OrderedNotifier for OrderedNotifierAdapter {
    fn send_ordered_nodes(
        &self,
        mut ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) {
        {
            let anchor_round = ordered_nodes.last().unwrap().round();
            let last = self.buffer.lock().last().cloned();
            if let Some(last) = last {
                if last.round() == anchor_round {
                    self.buffer.lock().append(&mut ordered_nodes);
                    return;
                } else {
                    mem::swap(&mut *self.buffer.lock(), &mut ordered_nodes);
                    self.buffer.lock().reserve(100);
                }
            } else {
                self.buffer.lock().append(&mut ordered_nodes);
                return;
            }
        }

        BLOCK_COUNTER
            .with_label_values(&[&self.dag_id.to_string()])
            .inc();

        let anchor = ordered_nodes.last().unwrap();
        let round = anchor.round();

        let idx = self.idx_gen.lock().next(round);
        assert!(idx < (self.epoch_state.verifier.len() as u64));
        let dag_external_round = (round * self.epoch_state.verifier.len() as Round) + idx;

        let block_info = ShoalppOrderBlocksInfo {
            dag_id: self.dag_id,
            ordered_nodes,
            failed_author,
            idx,
            dag_external_round,
        };
        if self.executor_channel.send(block_info).is_err() {
            error!("[DAG] execution pipeline closed");
        }
    }
}

pub struct StorageAdapter {
    epoch: u64,
    epoch_to_validators: HashMap<u64, Vec<Author>>,
    consensus_db: Arc<ConsensusDB>,
    aptos_db: Arc<dyn DbReader>,
    _reporter: RocksdbPropertyReporter,
}

impl StorageAdapter {
    pub fn new(
        epoch: u64,
        epoch_to_validators: HashMap<u64, Vec<Author>>,
        consensus_db: Arc<ConsensusDB>,
        aptos_db: Arc<dyn DbReader>,
    ) -> Self {
        Self {
            _reporter: RocksdbPropertyReporter::new(consensus_db.clone()),
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

    fn get_commit_history_resource(
        &self,
        latest_version: u64,
    ) -> anyhow::Result<CommitHistoryResource> {
        Ok(bcs::from_bytes(
            self.aptos_db
                .get_state_value_by_version(
                    &StateKey::on_chain_config::<CommitHistoryResource>(),
                    latest_version,
                )?
                .ok_or_else(|| format_err!("Resource doesn't exist"))?
                .bytes(),
        )?)
    }
}

impl DAGStorage for StorageAdapter {
    fn save_pending_node(&self, node: &Node, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self.consensus_db.put::<Dag0NodeSchema>(&(), node)?),
            1 => Ok(self.consensus_db.put::<Dag1NodeSchema>(&(), node)?),
            2 => Ok(self.consensus_db.put::<Dag2NodeSchema>(&(), node)?),
            _ => {
                unreachable!()
            },
        }
    }

    fn get_pending_node(&self, dag_id: u8) -> anyhow::Result<Option<Node>> {
        match dag_id {
            0 => Ok(self.consensus_db.get::<Dag0NodeSchema>(&())?),
            1 => Ok(self.consensus_db.get::<Dag1NodeSchema>(&())?),
            2 => Ok(self.consensus_db.get::<Dag2NodeSchema>(&())?),
            _ => {
                unreachable!()
            },
        }
    }

    fn delete_pending_node(&self, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self.consensus_db.delete::<Dag0NodeSchema>(vec![()])?),
            1 => Ok(self.consensus_db.delete::<Dag1NodeSchema>(vec![()])?),
            2 => Ok(self.consensus_db.delete::<Dag2NodeSchema>(vec![()])?),
            _ => {
                unreachable!()
            },
        }
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self.consensus_db.put::<Dag0VoteSchema>(node_id, vote)?),
            1 => Ok(self.consensus_db.put::<Dag1VoteSchema>(node_id, vote)?),
            2 => Ok(self.consensus_db.put::<Dag2VoteSchema>(node_id, vote)?),
            _ => {
                unreachable!()
            },
        }
    }

    fn get_votes(&self, dag_id: u8) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        match dag_id {
            0 => Ok(self.consensus_db.get_all::<Dag0VoteSchema>()?),
            1 => Ok(self.consensus_db.get_all::<Dag1VoteSchema>()?),
            2 => Ok(self.consensus_db.get_all::<Dag2VoteSchema>()?),
            _ => {
                unreachable!()
            },
        }
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self.consensus_db.delete::<Dag0VoteSchema>(node_ids)?),
            1 => Ok(self.consensus_db.delete::<Dag1VoteSchema>(node_ids)?),
            2 => Ok(self.consensus_db.delete::<Dag2VoteSchema>(node_ids)?),
            _ => {
                unreachable!()
            },
        }
    }

    fn save_certified_node(&self, node: &CertifiedNode, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self
                .consensus_db
                .put::<Dag0CertifiedNodeSchema>(&node.digest(), node)?),
            1 => Ok(self
                .consensus_db
                .put::<Dag1CertifiedNodeSchema>(&node.digest(), node)?),
            2 => Ok(self
                .consensus_db
                .put::<Dag2CertifiedNodeSchema>(&node.digest(), node)?),
            _ => {
                unreachable!()
            },
        }
    }

    fn get_certified_nodes(&self, dag_id: u8) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        match dag_id {
            0 => Ok(self.consensus_db.get_all::<Dag0CertifiedNodeSchema>()?),
            1 => Ok(self.consensus_db.get_all::<Dag1CertifiedNodeSchema>()?),
            2 => Ok(self.consensus_db.get_all::<Dag2CertifiedNodeSchema>()?),
            _ => {
                unreachable!()
            },
        }
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>, dag_id: u8) -> anyhow::Result<()> {
        match dag_id {
            0 => Ok(self
                .consensus_db
                .delete::<Dag0CertifiedNodeSchema>(digests)?),
            1 => Ok(self
                .consensus_db
                .delete::<Dag1CertifiedNodeSchema>(digests)?),
            2 => Ok(self
                .consensus_db
                .delete::<Dag2CertifiedNodeSchema>(digests)?),
            _ => {
                unreachable!()
            },
        }
    }

    fn get_latest_k_committed_events(&self, k: u64) -> anyhow::Result<Vec<CommitEvent>> {
        let timer = counters::FETCH_COMMIT_HISTORY_DURATION.start_timer();
        let version = self.aptos_db.get_latest_version()?;
        let resource = self.get_commit_history_resource(version)?;
        let handle = resource.table_handle();
        let mut commit_events = vec![];
        return Ok(commit_events);
        for i in 1..=std::cmp::min(k, resource.length()) {
            let idx = (resource.next_idx() + resource.max_capacity() - i as u32)
                % resource.max_capacity();
            let new_block_event = bcs::from_bytes::<NewBlockEvent>(
                self.aptos_db
                    .get_state_value_by_version(
                        &StateKey::table_item(*handle, bcs::to_bytes(&idx).unwrap()),
                        version,
                    )?
                    .ok_or_else(|| format_err!("Table item doesn't exist"))?
                    .bytes(),
            )?;
            if self
                .epoch_to_validators
                .contains_key(&new_block_event.epoch())
            {
                commit_events.push(self.convert(new_block_event)?);
            }
        }
        let duration = timer.stop_and_record();
        info!("[DAG] fetch commit history duration: {} sec", duration);
        commit_events.reverse();
        Ok(commit_events)
    }

    fn get_latest_ledger_info(&self) -> anyhow::Result<LedgerInfoWithSignatures> {
        // TODO: use callback from notifier to cache the latest ledger info
        Ok(self.aptos_db.get_latest_ledger_info()?)
    }

    fn get_epoch_to_proposers(&self) -> HashMap<u64, Vec<Author>> {
        self.epoch_to_validators.clone()
    }
}

pub(crate) trait TLedgerInfoProvider: Send + Sync {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures;

    fn get_highest_committed_anchor_round(&self, dag_id: u8) -> Round;
}

pub struct LedgerInfoProvider {
    latest_ledger_info: LedgerInfoWithSignatures,
    epoch_state: Arc<EpochState>,
}

impl LedgerInfoProvider {
    pub(super) fn new(
        epoch_state: Arc<EpochState>,
        mut latest_ledger_info: LedgerInfoWithSignatures,
    ) -> Self {
        let ledger_info_epoch = latest_ledger_info.ledger_info().epoch();
        if epoch_state.epoch > ledger_info_epoch {
            // TODO: verify it does what I think it does.
            let committed_rounds = HashValue::new([0; HashValue::LENGTH]);
            latest_ledger_info.set_consensus_data_hash(committed_rounds);
        }
        Self {
            latest_ledger_info,
            epoch_state,
        }
    }

    pub(super) fn notify_commit_proof(&mut self, ledger_info: LedgerInfoWithSignatures) {
        self.latest_ledger_info = ledger_info;
    }

    fn get_highest_committed_shoalpp_round(&self, dag_id: u8) -> Round {
        let committed_anchor_rounds = self
            .latest_ledger_info
            .get_highest_committed_rounds_for_shoalpp();
        committed_anchor_rounds[dag_id as usize]
    }

    fn get_highest_committed_anchor_round(&self, dag_id: u8) -> Round {
        self.get_highest_committed_shoalpp_round(dag_id) / self.epoch_state.verifier.len() as u64
    }
}

impl TLedgerInfoProvider for RwLock<LedgerInfoProvider> {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures {
        self.read().latest_ledger_info.clone()
    }

    fn get_highest_committed_anchor_round(&self, dag_id: u8) -> Round {
        self.read().get_highest_committed_anchor_round(dag_id)
    }
}
