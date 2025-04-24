// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus;
use aptos_config::config::NodeConfig;
use aptos_consensus::monitor;
use aptos_consensus_notifications::{
    ConsensusNotification, ConsensusNotificationListener, ConsensusNotifier,
};
use aptos_crypto::HashValue;
use aptos_framework::natives::debug;
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_mempool::{QuorumStoreRequest, QuorumStoreResponse};
use aptos_mempool_notifications::{
    CommittedTransaction, MempoolCommitNotification, MempoolNotificationListener, MempoolNotifier,
};
use aptos_metrics_core::{register_gauge, register_histogram, Gauge, Histogram};
use aptos_types::{
    transaction::{SignedTransaction, Transaction},
    PeerId,
};
use futures::{
    channel::{
        mpsc::{Receiver, Sender},
        oneshot,
    },
    future::Pending,
    select, SinkExt, StreamExt,
};
use itertools::zip_eq;
use once_cell::sync::Lazy;
use std::{
    cmp::min,
    collections::{BTreeMap, HashMap, VecDeque},
    convert::Infallible,
    iter::zip,
    mem,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{runtime::Runtime, task::JoinHandle};
use warp::{
    filters::BoxedFilter,
    reject::Rejection,
    reply::{self, Reply},
    Filter,
};

pub type TransactionStore = VecDeque<(SignedTransaction, oneshot::Sender<()>, Instant)>;

struct TrackingItem {
    tx: oneshot::Sender<()>,
    insert_time: Instant,
}

pub struct PendingTracker {
    tracker: HashMap<(PeerId, u64), TrackingItem>,
}

impl PendingTracker {
    fn register(&mut self, txn: &SignedTransaction, tx: oneshot::Sender<()>, insert_time: Instant) {
        self.tracker
            .insert((txn.sender(), txn.sequence_number()), TrackingItem {
                tx,
                insert_time,
            });
    }

    fn notify(&mut self, txn: &Transaction) {
        let txn = txn.try_as_signed_user_txn().unwrap();
        if let Some(item) = self.tracker.remove(&(txn.sender(), txn.sequence_number())) {
            RAIKOU_MEMPOOL_INSERT_TO_COMMIT_LATENCY
                .observe(item.insert_time.elapsed().as_secs_f64());
            if item.tx.send(()).is_err() {
                error!("client timedout: {}", txn.sender());
            }
        }
    }
}

const COMMIT_LATENCY_BUCKETS: &[f64] = &[
    0.01, 0.02, 0.04, 0.06, 0.08, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65,
    0.7, 0.75, 0.8, 0.85, 0.9, 0.95, 1.0, 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.2,
    2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0, 4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 10.0,
];

pub static RAIKOU_MEMPOOL_PULL_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "raikou_mempool_insert_to_pull_latency",
        "Raikou Mempool Insert to Pull Latency",
        COMMIT_LATENCY_BUCKETS.to_vec(),
    )
    .unwrap()
});

pub static RAIKOU_MEMPOOL_INSERT_TO_COMMIT_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "raikou_mempool_insert_to_commit_latency",
        "Raikou Pull to Commit Latnecy",
        COMMIT_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static RAIKOU_MEMPOOL_SIZE: Lazy<Gauge> =
    Lazy::new(|| register_gauge!("raikou_mempool_size", "Raikou mempool size").unwrap());

pub static RAIKOU_MEMPOOL_SIZE_HISTOGRAM: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!("raikou_mempool_size_histogram", "raptr mempool size").unwrap()
});

pub struct BatchGenerator {
    // Track both mempool sizes and how much we pulled
    recent_states: VecDeque<(usize, usize)>, // (mempool_size, last_pull_size)
    window_size: usize,
    expected_batch_count: usize,
}

impl BatchGenerator {
    pub fn new() -> Self {
        let expected_batch_count = 5;
        let window_size = 5;

        Self {
            recent_states: VecDeque::with_capacity(window_size),
            window_size,
            expected_batch_count,
        }
    }

    pub fn calculate_next_batch_size(
        &mut self,
        max_batch_size: usize,
        current_mempool_size: usize,
    ) -> usize {
        // Handle empty mempool case
        if current_mempool_size == 0 {
            self.recent_states.push_back((0, 0));
            if self.recent_states.len() > self.window_size {
                self.recent_states.pop_front();
            }
            return 0;
        }

        if self.recent_states.is_empty() {
            // Simple division for first pull
            let batch_size = current_mempool_size / self.expected_batch_count;
            let batch_size = batch_size.max(1); // Ensure at least 1 transaction
            self.recent_states
                .push_back((current_mempool_size, batch_size));
            return batch_size;
        }

        // Case 3: Normal operation with history
        // Calculate net transaction flow for each interval
        let mut net_flow = 0i64;
        let mut prev_size = current_mempool_size;

        // Iterate through history in reverse (newest to oldest)
        for &(size, pulled) in self.recent_states.iter().rev() {
            // Calculate change in this interval:
            // current_size = prev_size + new_txns - pulled
            // therefore: new_txns = current_size - prev_size + pulled
            let interval_change = (prev_size as i64)
                .saturating_sub(size as i64)
                .saturating_add(pulled as i64);

            net_flow = net_flow + interval_change;
            prev_size = size;
        }

        // Calculate average flow per interval
        let intervals = self.recent_states.len() as f64;
        let avg_flow = (net_flow as f64 / intervals).max(0.0);

        // Scale based on how full our history window is
        let scaling_factor = intervals / self.window_size as f64;
        let expected_incoming_txns = (avg_flow * scaling_factor).round() as usize;

        // Calculate final batch size safely
        let total_to_process = current_mempool_size + expected_incoming_txns;

        let batch_size = total_to_process
            .checked_div(self.expected_batch_count)
            .unwrap_or(1);
        let batch_size = batch_size.clamp(100.min(current_mempool_size), max_batch_size);

        // Update our history
        self.recent_states
            .push_back((current_mempool_size, batch_size));
        if self.recent_states.len() > self.window_size {
            self.recent_states.pop_front();
        }

        batch_size
    }
}

pub struct SimpleMempool {
    pub transaction_store: TransactionStore,
    pub pending_tracker: PendingTracker,
    pub generator: BatchGenerator,
}

impl SimpleMempool {
    fn handle_quorum_store_request(&mut self, req: QuorumStoreRequest) {
        let QuorumStoreRequest::GetBatchRequest(max_txns, max_bytes, _, _, sender) = req else {
            unreachable!();
        };
        let mut store = &mut self.transaction_store;

        info!("pulling: store len: {}", store.len());

        // let to_pull = self
        //     .generator
        //     .calculate_next_batch_size(max_txns as usize, store.len());
        // let to_pull = to_pull.min(store.len());
        let to_pull = min(store.len(), max_txns as usize);
        let pulled = store.drain(..to_pull);
        let mut pulled_txns = Vec::with_capacity(to_pull);

        for (txn, tx, insert_time) in pulled {
            RAIKOU_MEMPOOL_PULL_LATENCY.observe(insert_time.elapsed().as_secs_f64());
            self.pending_tracker.register(&txn, tx, insert_time);
            pulled_txns.push(txn);
        }
        if pulled_txns.is_empty() {
            sample!(
                SampleRate::Duration(Duration::from_secs(10)),
                info!("pulled_txns empty");
            );
        } else {
            sample!(
                SampleRate::Duration(Duration::from_secs(10)),
                info!(
                    "pulled txns: {}; store len: {}",
                    pulled_txns.len(),
                    store.len()
                )
            );
        }
        RAIKOU_MEMPOOL_SIZE.set(store.len() as f64);
        RAIKOU_MEMPOOL_SIZE_HISTOGRAM.observe(store.len() as f64);

        sender
            .send(Ok(QuorumStoreResponse::GetBatchResponse(pulled_txns)))
            .unwrap();
    }

    fn handle_notification_command(
        &mut self,
        cmd: ConsensusNotification,
        listener: &mut ConsensusNotificationListener,
    ) {
        let ConsensusNotification::NotifyCommit(notif) = cmd else {
            return;
        };
        for txn in notif.get_transactions() {
            self.pending_tracker.notify(txn);
        }
        listener
            .respond_to_commit_notification(notif, Ok(()))
            .unwrap();
    }

    fn handle_txn_request(&mut self, req: (SignedTransaction, oneshot::Sender<()>, Instant)) {
        self.transaction_store.push_back(req);
    }

    async fn run(
        mut self,
        mut consensus_rx: Receiver<QuorumStoreRequest>,
        mut commit_notif_rx: ConsensusNotificationListener,
        mut api_rx: Receiver<(SignedTransaction, oneshot::Sender<()>, Instant)>,
    ) {
        loop {
            monitor!("mempool_runloop", tokio::select! {
                biased;
                Some(req) = consensus_rx.next() => {
                    monitor!("mempool_qsr", self.handle_quorum_store_request(req));
                },
                Some(req) = api_rx.next() => {
                    monitor!("mempool_apirx", self.handle_txn_request(req));
                },
                Some(cmd) = commit_notif_rx.next() => {
                    monitor!("mempool_notif", self.handle_notification_command(cmd, &mut commit_notif_rx));
                },
            })
        }
    }
}

pub fn create_mempool_runtime(
    config: &NodeConfig,
    health_check: Arc<AtomicBool>,
) -> (Runtime, Sender<QuorumStoreRequest>, ConsensusNotifier) {
    let (consensus_to_mempool_sender, consensus_to_mempool_receiver) =
        futures::channel::mpsc::channel(100);

    let (consensus_notifier, consensus_listener) =
        aptos_consensus_notifications::new_consensus_notifier_listener_pair(1000);

    let runtime = aptos_runtimes::spawn_named_runtime("mempool".into(), None);

    let store = TransactionStore::with_capacity(200_000);
    let mempool = SimpleMempool {
        transaction_store: store,
        pending_tracker: PendingTracker {
            tracker: HashMap::with_capacity(100_000),
        },
        generator: BatchGenerator::new(),
    };

    let (api_mempool_tx, api_mempool_rx) = futures::channel::mpsc::channel(5_000);
    runtime.spawn(mempool.run(
        consensus_to_mempool_receiver,
        consensus_listener,
        api_mempool_rx,
    ));
    runtime.spawn(start_mempool_api(
        health_check,
        api_mempool_tx,
        config.api.address,
    ));

    (runtime, consensus_to_mempool_sender, consensus_notifier)
}

#[derive(Clone)]
pub struct SimpleMempoolApiContext {
    mempool_tx: Sender<(SignedTransaction, oneshot::Sender<()>, Instant)>,
    health_check: Arc<AtomicBool>,
}

impl SimpleMempoolApiContext {
    pub fn filter(
        self,
    ) -> impl Filter<Extract = (SimpleMempoolApiContext,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }
}

async fn start_mempool_api(
    health_check: Arc<AtomicBool>,
    mempool_tx: Sender<(SignedTransaction, oneshot::Sender<()>, Instant)>,
    api_address: SocketAddr,
) {
    let context = SimpleMempoolApiContext {
        health_check,
        mempool_tx,
    };
    let ctx_filter = context.filter().clone();

    let submit = warp::path!("submit_txn")
        .and(warp::post())
        .and(ctx_filter.clone())
        .and(warp::body::bytes())
        .and_then(handle_txn)
        .boxed();

    let submit_batch = warp::path!("submit_txn_batch")
        .and(warp::post())
        .and(ctx_filter.clone())
        .and(warp::body::bytes())
        .and_then(handle_txn_batch)
        .boxed();

    let health_check = warp::path!("health_check")
        .and(warp::get())
        .and(ctx_filter)
        .and_then(handle_health_check)
        .boxed();

    let api = submit.or(health_check).or(submit_batch);

    warp::serve(api).bind(api_address).await
}

pub async fn handle_txn(
    context: SimpleMempoolApiContext,
    request: bytes::Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    if !context.health_check.load(Ordering::Relaxed) {
        return Err(warp::reject::not_found());
    }

    let txn: SignedTransaction = bcs::from_bytes(&request).unwrap();
    let (tx, rx) = oneshot::channel();

    let mut mempool_tx = context.mempool_tx.clone();
    mempool_tx.send((txn, tx, Instant::now())).await.ok();

    if let Err(_) = rx.await {
        // debug!("api: response channel dropped unexpectedly");
        return Ok(reply::with_status(
            reply::reply(),
            warp::hyper::StatusCode::BAD_REQUEST,
        ));
    }

    // debug!("api: txn successfully committed");
    Ok(reply::with_status(
        reply::reply(),
        warp::hyper::StatusCode::CREATED,
    ))
}

pub async fn handle_health_check(
    context: SimpleMempoolApiContext,
) -> anyhow::Result<impl Reply, Rejection> {
    if context.health_check.load(Ordering::Relaxed) {
        Ok(reply::with_status(
            reply::reply(),
            warp::hyper::StatusCode::CREATED,
        ))
    } else {
        Err(warp::reject::not_found())
    }
}

pub async fn handle_txn_batch(
    context: SimpleMempoolApiContext,
    request: bytes::Bytes,
) -> anyhow::Result<impl Reply, Rejection> {
    if !context.health_check.load(Ordering::Relaxed) {
        return Err(warp::reject::not_found());
    }

    let txn_batch: Vec<SignedTransaction> = bcs::from_bytes(&request).unwrap();

    let now = Instant::now();
    let mut rxs = Vec::new();
    let mut mempool_tx = context.mempool_tx.clone();
    for txn in txn_batch {
        let (tx, rx) = oneshot::channel();
        mempool_tx.send((txn, tx, Instant::now())).await.ok();
        rxs.push(rx);
    }

    // for rx in rxs {
    //     if let Err(_) = rx.await {
    //         debug!("api: response channel dropped unexpectedly");
    //         return Ok(reply::with_status(
    //             reply::reply(),
    //             warp::hyper::StatusCode::BAD_REQUEST,
    //         ));
    //     }
    // }

    // debug!("api: batch txn successfully committed");
    Ok(reply::with_status(
        reply::reply(),
        warp::hyper::StatusCode::CREATED,
    ))
}
