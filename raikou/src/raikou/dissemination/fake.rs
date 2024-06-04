use crate::{
    framework::{network::NetworkService, timer::TimerService, NodeId, Protocol},
    protocol,
    raikou::{dissemination::DisseminationLayer, types::*},
};
use aptos_consensus_types::common::PayloadFilter;
use bitvec::prelude::BitVec;
use defaultmap::DefaultBTreeMap;
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    future::Future,
    sync::Arc,
    time::Duration,
};
use tokio::time::Instant;

#[derive(Clone)]
pub struct Batch {
    node: NodeId,
    sn: BatchSN,
    txns: Option<Vec<Txn>>,
}

impl Batch {
    pub fn get_ref(&self) -> BatchInfo {
        BatchInfo {
            node: self.node,
            sn: self.sn,
        }
    }
}

#[derive(Clone)]
pub enum Message {
    Batch(Batch),
    BatchStored(BatchSN),
    AvailabilityCert(AC),
    Fetch(BatchInfo),
}

#[derive(Clone)]
pub enum TimerEvent {
    NewBatch(BatchSN),
}

pub struct Config {
    pub n_nodes: usize,
    pub ac_quorum: usize,
    pub batch_interval: Duration,
}

#[derive(Clone)]
pub struct FakeDisseminationLayer<TI> {
    inner: Arc<tokio::sync::Mutex<FakeDisseminationLayerInner<TI>>>,
}

impl<TI> FakeDisseminationLayer<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    pub fn new(node_id: NodeId, config: Config, txns_iter: TI) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(FakeDisseminationLayerInner::new(
                node_id, config, txns_iter,
            ))),
        }
    }

    pub fn protocol(
        &self,
    ) -> Arc<tokio::sync::Mutex<impl Protocol<Message = Message, TimerEvent = TimerEvent>>> {
        self.inner.clone()
    }
}

impl<TI> DisseminationLayer for FakeDisseminationLayer<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync + 'static,
{
    async fn pull_payload(&self, exclude: HashSet<BatchInfo>) -> Payload {
        let inner = self.inner.lock().await;

        let acs = inner
            .new_acs
            .iter()
            .filter(|&batch_info| !exclude.contains(batch_info))
            .map(|batch_info| inner.acs[batch_info].clone())
            .collect();

        let batches = inner
            .new_batches
            .iter()
            .filter(|&batch_info| !exclude.contains(batch_info))
            .cloned()
            .collect();

        Payload::new(acs, batches)
    }

    async fn prefetch_payload_data(&self, payload: Payload) {
        let new_acs = payload
            .acs()
            .into_iter()
            .cloned()
            .map(|ac| (ac.batch.clone(), ac));
        self.inner.lock().await.acs.extend(new_acs);
    }

    async fn check_stored(&self, batch: &BatchInfo) -> bool {
        self.inner.lock().await.batches.contains_key(batch)
    }

    async fn notify_commit(&self, payloads: Vec<Payload>) {
        let mut inner = self.inner.lock().await;

        // TODO: replace if with an assert once deduplication is implemented.
        for payload in payloads {
            for batch in payload.all() {
                if !inner.committed_batches.contains(batch) {
                    inner.committed_batches.insert(batch.clone());
                    inner.new_acs.remove(batch);
                    inner.new_batches.remove(batch);
                }
            }
        }

        // TODO: add commit time metric?
        // if batch_ref.node == self.node_id {
        //     let commit_time = self.batch_created_time[batch_ref.sn]
        //         .elapsed()
        //         .as_secs_f64()
        //         / self.config.delta.as_secs_f64();
        //     self.metrics
        //         .batch_commit_time
        //         .push((self.batch_created_time[batch_ref.sn], commit_time));
        // }
    }
}

pub struct FakeDisseminationLayerInner<TI> {
    txns_iter: TI,
    config: Config,
    node_id: NodeId,

    // Storage for all received batches and the time when they were.
    batches: BTreeMap<BatchInfo, Batch>,
    // Storage of all received ACs.
    acs: BTreeMap<BatchInfo, AC>,
    // Set of committed batches.
    committed_batches: BTreeSet<BatchInfo>,
    // Set of known ACs that are not yet committed.
    new_acs: BTreeSet<BatchInfo>,
    // Set of known uncertified batches that are not yet committed.
    new_batches: BTreeSet<BatchInfo>,

    // The set of nodes that have stored this node's batch with the given sequence number.
    batch_stored_votes: DefaultBTreeMap<BatchSN, BitVec>,

    batch_created_time: DefaultBTreeMap<BatchSN, Instant>,
}

impl<TI> FakeDisseminationLayerInner<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    pub fn new(node_id: NodeId, config: Config, txns_iter: TI) -> Self {
        let n_nodes = config.n_nodes;

        Self {
            txns_iter,
            config,
            node_id,
            batches: BTreeMap::new(),
            acs: BTreeMap::new(),
            committed_batches: BTreeSet::new(),
            new_acs: BTreeSet::new(),
            new_batches: BTreeSet::new(),
            batch_stored_votes: DefaultBTreeMap::new(BitVec::repeat(false, n_nodes)),
            batch_created_time: DefaultBTreeMap::new(Instant::now()),
        }
    }
}

impl<TI> Protocol for FakeDisseminationLayerInner<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    type Message = Message;
    type TimerEvent = TimerEvent;

    protocol! {
        self: self;
        ctx: ctx;

        // Dissemination layer
        // In this implementation, batches are simply sent periodically, by a timer.

        upon start {
            // The first batch is sent immediately.
            ctx.set_timer(Duration::ZERO, TimerEvent::NewBatch(1));
        };

        upon timer [TimerEvent::NewBatch(sn)] {
            // Multicast a new batch
            ctx.multicast(Message::Batch(Batch {
                node: self.node_id,
                sn,
                txns: self.txns_iter.next(),
            })).await;

            self.batch_created_time[sn] = Instant::now();

            // Reset the timer.
            ctx.set_timer(self.config.batch_interval, TimerEvent::NewBatch(sn + 1));
        };

        // Upon receiving a batch, store it, reply with a BatchStored message,
        // and execute try_vote.
        upon receive [Message::Batch(batch)] from node [p] {
            let batch_ref = batch.get_ref();
            if !self.batches.contains_key(&batch_ref) {
                self.batches.insert(batch_ref, batch);

                // TODO
                // self.penalty_tracker.on_new_batch(batch_ref);

                ctx.unicast(Message::BatchStored(batch_ref.sn), p).await;

                // Track the list of known uncommitted uncertified batches.
                if !self.acs.contains_key(&batch_ref) && !self.committed_batches.contains(&batch_ref) {
                    self.new_batches.insert(batch_ref);
                }
            }
        };

        // Upon receiving a quorum of BatchStored messages for a batch,
        // form an AC and broadcast it.
        upon receive [Message::BatchStored(sn)] from node [p] {
            self.batch_stored_votes[sn].set(p, true);

            if self.batch_stored_votes[sn].count_ones() == self.config.ac_quorum {
                ctx.multicast(Message::AvailabilityCert(AC {
                    batch: BatchInfo { node: self.node_id, sn },
                    signers: self.batch_stored_votes[sn].clone(),
                })).await;
            }
        };

        upon receive [Message::AvailabilityCert(ac)] from [_any_node] {
            self.acs.insert(ac.batch, ac.clone());

            // Track the list of known uncommitted ACs
            // and the list of known uncommitted uncertified batches.
            if !self.committed_batches.contains(&ac.batch) {
                self.new_acs.insert(ac.batch);
                self.new_batches.remove(&ac.batch);
            }
        };

        upon receive [Message::Fetch(batch_ref)] from node [p] {
            // FIXME: fetching is not actually being used yet.
            //        `Message::Fetch` is never sent.
            // If receive a Fetch message, reply with the batch if it is known.
            if let Some(batch) = self.batches.get(&batch_ref) {
                ctx.unicast(Message::Batch(batch.clone()), p).await;
            }
        };
    }
}
