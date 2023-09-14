// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensusdb::{CertifiedNodeSchema, ConsensusDB, DagVoteSchema, NodeSchema},
    counters::update_counters_for_committed_blocks,
    dag::{
        storage::{CommitEvent, DAGStorage},
        CertifiedNode, Node, NodeId, Vote,
    },
    experimental::buffer_manager::OrderedBlocks,
};
use anyhow::{anyhow, bail};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, Round},
    executed_block::ExecutedBlock,
    quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_storage_interface::{DbReader, Order};
use aptos_types::{
    account_config::{new_block_event_key, NewBlockEvent},
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use async_trait::async_trait;
use futures_channel::mpsc::UnboundedSender;
use std::{collections::HashMap, sync::Arc};

pub trait OrderedNotifier: Send + Sync {
    fn send_ordered_nodes(
        &self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()>;
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
    executor_channel: UnboundedSender<OrderedBlocks>,
    storage: Arc<dyn DAGStorage>,
    parent_block_info: Arc<RwLock<BlockInfo>>,
    epoch_state: Arc<EpochState>,
    ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
}

impl OrderedNotifierAdapter {
    pub(super) fn new(
        executor_channel: UnboundedSender<OrderedBlocks>,
        storage: Arc<dyn DAGStorage>,
        epoch_state: Arc<EpochState>,
        parent_block_info: BlockInfo,
        ledger_info_provider: Arc<RwLock<LedgerInfoProvider>>,
    ) -> Self {
        Self {
            executor_channel,
            storage,
            parent_block_info: Arc::new(RwLock::new(parent_block_info)),
            epoch_state,
            ledger_info_provider,
        }
    }
}

impl OrderedNotifier for OrderedNotifierAdapter {
    fn send_ordered_nodes(
        &self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()> {
        let anchor = ordered_nodes.last().unwrap();
        let epoch = anchor.epoch();
        let round = anchor.round();
        let timestamp = anchor.metadata().timestamp();
        let author = *anchor.author();
        let mut payload = Payload::empty(!anchor.payload().is_direct());
        let mut node_digests = vec![];
        for node in &ordered_nodes {
            payload.extend(node.payload().clone());
            node_digests.push(node.digest());
        }
        let parent_block_id = self.parent_block_info.read().id();
        // construct the bitvec that indicates which nodes present in the previous round in CommitEvent
        let mut parents_bitvec = BitVec::with_num_bits(self.epoch_state.verifier.len() as u16);
        for parent in anchor.parents().iter() {
            if let Some(idx) = self
                .epoch_state
                .verifier
                .address_to_validator_index()
                .get(parent.metadata().author())
            {
                parents_bitvec.set(*idx as u16);
            }
        }

        let block = ExecutedBlock::new(
            Block::new_for_dag(
                epoch,
                round,
                timestamp,
                payload,
                author,
                failed_author,
                parent_block_id,
                parents_bitvec,
                node_digests,
            ),
            StateComputeResult::new_dummy(),
        );
        let block_info = block.block_info();
        let storage = self.storage.clone();
        let ledger_info_provider = self.ledger_info_provider.clone();
        *self.parent_block_info.write() = block_info.clone();
        Ok(self.executor_channel.unbounded_send(OrderedBlocks {
            ordered_blocks: vec![block],
            ordered_proof: LedgerInfoWithSignatures::new(
                LedgerInfo::new(block_info, anchor.digest()),
                AggregateSignature::empty(),
            ),
            callback: Box::new(
                move |committed_blocks: &[Arc<ExecutedBlock>],
                      commit_decision: LedgerInfoWithSignatures| {
                    ledger_info_provider
                        .write()
                        .notify_commit_proof(commit_decision);
                    update_counters_for_committed_blocks(committed_blocks);
                    for executed_block in committed_blocks {
                        if let Some(node_digests) = executed_block.block().block_data().dag_nodes()
                        {
                            if let Err(e) = storage.delete_certified_nodes(node_digests.clone()) {
                                error!(
                                    "Failed to garbage collect committed for block {}: {:?}",
                                    executed_block.block(),
                                    e
                                );
                            }
                        }
                    }
                },
            ),
        })?)
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
        let latest_db_version = self.aptos_db.get_latest_version().unwrap_or(0);
        let mut commit_events = vec![];
        for event in self.aptos_db.get_events(
            &new_block_event_key(),
            u64::MAX,
            Order::Descending,
            k,
            latest_db_version,
        )? {
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
        self.aptos_db.get_latest_ledger_info()
    }
}

pub(crate) trait TLedgerInfoProvider: Send + Sync {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures;

    fn get_highest_committed_anchor_round(&self) -> Round;
}

pub(super) struct LedgerInfoProvider {
    latest_ledger_info: LedgerInfoWithSignatures,
}

impl LedgerInfoProvider {
    pub(super) fn new(latest_ledger_info: LedgerInfoWithSignatures) -> Self {
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

    fn get_highest_committed_anchor_round(&self) -> Round {
        self.read().latest_ledger_info.ledger_info().round()
    }
}
