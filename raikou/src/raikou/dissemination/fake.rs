use crate::{
    framework::{network::NetworkService, timer::TimerService, NodeId, Protocol},
    protocol,
    raikou::{dissemination::DisseminationLayer, types::*},
};
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
    author: NodeId,
    batch_id: BatchId,
    digest: HashValue,
    txns: Option<Vec<Txn>>,
}

impl Batch {
    pub fn get_info(&self) -> BatchInfo {
        BatchInfo {
            author: self.author,
            batch_id: self.batch_id,
            digest: self.digest,
        }
    }
}

#[derive(Clone)]
pub enum Message {
    Batch(Batch),
    BatchStored(BatchId),
    AvailabilityCert(AC),
    // Fetch(BatchHash),
}

#[derive(Clone)]
pub enum TimerEvent {
    NewBatch(BatchId),
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
    async fn prepare_block(&self, exclude: HashSet<BatchHash>) -> Payload {
        let inner = self.inner.lock().await;

        let acs = inner
            .new_acs
            .iter()
            .filter(|&batch_hash| !exclude.contains(batch_hash))
            .map(|batch_hash| inner.acs[batch_hash].clone())  // WARNING: potentially expensive clone
            .collect();

        let batches = inner
            .new_batches
            .iter()
            .filter(|&batch_hash| !exclude.contains(batch_hash))
            .map(|batch_hash| inner.batches[batch_hash].get_info())  // WARNING: potentially expensive clone
            .collect();

        Payload::new(acs, batches)
    }

    async fn prefetch_payload_data(&self, payload: Payload) {
        let new_acs = payload
            .acs()
            .into_iter()
            .cloned()
            .map(|ac| (ac.batch.digest, ac));
        self.inner.lock().await.acs.extend(new_acs);
    }

    async fn check_stored(&self, batch: &BatchHash) -> bool {
        self.inner.lock().await.batches.contains_key(batch)
    }

    async fn notify_commit(&self, payloads: Vec<Payload>) {
        let mut inner = self.inner.lock().await;

        // TODO: replace if with an assert once deduplication is implemented.
        for payload in payloads {
            for batch in payload.all() {
                if !inner.committed_batches.contains(&batch.digest) {
                    inner.committed_batches.insert(batch.digest);
                    inner.new_acs.remove(&batch.digest);
                    inner.new_batches.remove(&batch.digest);
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
    batches: BTreeMap<BatchHash, Batch>,
    my_batches: BTreeMap<BatchId, BatchHash>,
    // Storage of all received ACs.
    acs: BTreeMap<BatchHash, AC>,
    // Set of committed batches.
    committed_batches: BTreeSet<BatchHash>,
    // Set of known ACs that are not yet committed.
    new_acs: BTreeSet<BatchHash>,
    // Set of known uncertified batches that are not yet committed.
    new_batches: BTreeSet<BatchHash>,

    // The set of nodes that have stored this node's batch with the given sequence number.
    batch_stored_votes: DefaultBTreeMap<BatchId, BitVec>,

    batch_created_time: DefaultBTreeMap<BatchId, Instant>,
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
            my_batches: Default::default(),
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
            let txns = self.txns_iter.next();
            let digest = hash((self.node_id, sn, &txns));

            // Multicast a new batch
            ctx.multicast(Message::Batch(Batch {
                author: self.node_id,
                batch_id: sn,
                digest,
                txns,
            })).await;

            self.batch_created_time[sn] = Instant::now();
            self.my_batches.insert(sn, digest);

            // Reset the timer.
            ctx.set_timer(self.config.batch_interval, TimerEvent::NewBatch(sn + 1));
        };

        // Upon receiving a batch, store it, reply with a BatchStored message,
        // and execute try_vote.
        upon receive [Message::Batch(batch)] from node [p] {
            // TODO: add verification of the digest?
            let digest = batch.digest;
            let batch_id = batch.batch_id;

            if !self.batches.contains_key(&digest) {
                self.batches.insert(digest, batch);

                // TODO
                // self.penalty_tracker.on_new_batch(batch_ref);

                ctx.unicast(Message::BatchStored(batch_id), p).await;

                // Track the list of known uncommitted uncertified batches.
                if !self.acs.contains_key(&digest) && !self.committed_batches.contains(&digest) {
                    self.new_batches.insert(digest);
                }
            }
        };

        // Upon receiving a quorum of BatchStored messages for a batch,
        // form an AC and broadcast it.
        upon receive [Message::BatchStored(batch_id)] from node [p] {
            self.batch_stored_votes[batch_id].set(p, true);

            if self.batch_stored_votes[batch_id].count_ones() == self.config.ac_quorum {
                let digest = self.my_batches[&batch_id];
                ctx.multicast(Message::AvailabilityCert(AC {
                    batch: self.batches[&digest].get_info(),
                    signers: self.batch_stored_votes[batch_id].clone(),
                })).await;
            }
        };

        upon receive [Message::AvailabilityCert(ac)] from [_any_node] {
            // Track the list of known uncommitted ACs
            // and the list of known uncommitted uncertified batches.
            if !self.committed_batches.contains(&ac.batch.digest) {
                self.new_acs.insert(ac.batch.digest);
                self.new_batches.remove(&ac.batch.digest);
            }

            self.acs.insert(ac.batch.digest, ac.clone());
        };

        // upon receive [Message::Fetch(digest)] from node [p] {
        //     // FIXME: fetching is not actually being used yet.
        //     //        `Message::Fetch` is never sent.
        //     // If receive a Fetch message, reply with the batch if it is known.
        //     if let Some(batch) = self.batches.get(&digest) {
        //         ctx.unicast(Message::Batch(batch.clone()), p).await;
        //     }
        // };
    }
}
