// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController, OutboundRpcScheduler};
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::{net::SocketAddr, sync::{Arc, RwLock}, thread};
use std::ops::DerefMut;
use std::time::SystemTime;

extern crate itertools;
use crate::metrics::REMOTE_EXECUTOR_TIMER;
use aptos_logger::{info, trace};
use aptos_types::state_store::{StateView, TStateView};
use itertools::Itertools;
use std::sync::{Condvar, Mutex};
use cpq::ConcurrentPriorityQueue;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio::runtime;
use tokio::runtime::Runtime;
use tokio::sync::Notify;
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER;


// Implement multi-producer multi-consumer async channel with priority queue
struct MpmcCpq<T> {
    messages: Mutex<ConcurrentPriorityQueue<T, u64>>,
    notify_on_sent: Notify,
}

impl<T> MpmcCpq<T> {
    pub fn send(&self, msg: T, priority: u64) {
        let mut locked_queue = self.messages.lock().unwrap();
        locked_queue.push(msg, priority);
        drop(locked_queue);

        self.notify_on_sent.notify_one();
    }

    pub fn len(&self) -> usize {
        let mut locked_queue = self.messages.lock().unwrap();
        return locked_queue.len();
    }
    pub fn try_recv(&self) -> Option<T> {
        let mut locked_queue = self.messages.lock().unwrap();
        locked_queue.pop()
    }

    pub async fn recv(&self) -> T {
        let future = self.notify_on_sent.notified();
        tokio::pin!(future);
        loop {
            future.as_mut().enable();

            if let Some(msg) = self.try_recv() {
                return msg;
            }

            future.as_mut().await;

            future.set(self.notify_on_sent.notified());
        }
    }
}

pub struct RemoteStateViewService<S: StateView + Sync + Send + 'static> {
    kv_rx: Receiver<Message>,
    kv_unprocessed_pq: Arc<ConcurrentPriorityQueue<Message, u64>>,
    kv_tx: Arc<Vec<Vec<Arc<tokio::sync::Mutex<OutboundRpcHelper>>>>>,
    //thread_pool: Arc<Vec<rayon::ThreadPool>>,
    //thread_pool: Arc<rayon::ThreadPool>,
    state_view: Arc<RwLock<Option<Arc<S>>>>,
    recv_condition: Arc<(tokio::sync::Mutex<bool>, Condvar)>,
    outbound_rpc_scheduler: Arc<OutboundRpcScheduler>,
    kv_proc_runtime: Arc<Runtime>,
    num_shards: usize,

}

impl<S: StateView + Sync + Send + 'static> RemoteStateViewService<S> {
    pub fn new(
        controller: &mut NetworkController,
        remote_shard_addresses: Vec<SocketAddr>,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = 70;//num_threads.unwrap_or_else(num_cpus::get) / 2 + 30;
        let num_kv_req_threads = 30; //num_cpus::get() / 2;
        let num_shards = remote_shard_addresses.len();
        info!("num threads for remote state view service: {}", num_threads);

        let kv_proc_rt = runtime::Builder::new_multi_thread().worker_threads(num_threads).enable_all().thread_name("kv_proc").build().unwrap();
        let kv_request_type = "remote_kv_request";
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_request_type.to_string());
        let command_txs = remote_shard_addresses
            .iter()
            .map(|address| {
                let mut command_tx = vec![];
                for _ in 0..num_kv_req_threads {
                    command_tx.push(Arc::new(tokio::sync::Mutex::new(OutboundRpcHelper::new(controller.get_self_addr(), *address, controller.get_outbound_rpc_runtime()))));
                }
                command_tx
            })
            .collect_vec();
        Self {
            kv_rx: result_rx,
            kv_unprocessed_pq: Arc::new(ConcurrentPriorityQueue::new()),
            kv_tx: Arc::new(command_txs),
            //thread_pool: Arc::new(thread_pool),
            state_view: Arc::new(RwLock::new(None)),
            recv_condition: Arc::new((tokio::sync::Mutex::new(false), Condvar::new())),
            outbound_rpc_scheduler: controller.get_outbound_rpc_scheduler(),
            kv_proc_runtime: Arc::new(kv_proc_rt),
            num_shards,
        }
    }

    pub fn set_state_view(&self, state_view: Arc<S>) {
        let mut state_view_lock = self.state_view.write().unwrap();
        *state_view_lock = Some(state_view);
    }

    pub fn drop_state_view(&self) {
        let mut state_view_lock = self.state_view.write().unwrap();
        *state_view_lock = None;
    }

    pub fn start(&self) {
        let kv_unprocessed = Arc::new(MpmcCpq {
            messages: Mutex::new(ConcurrentPriorityQueue::new()),
            notify_on_sent: Notify::new(),
        });
        let num_workers = self.kv_proc_runtime.handle().metrics().num_workers();

        info!("Num handlers created is {}", num_workers);
        for _ in 0..num_workers {
            let state_view_clone = self.state_view.clone();
            let kv_tx_clone = self.kv_tx.clone();
            let kv_unprocessed_clone = kv_unprocessed.clone();
            let recv_condition_clone = self.recv_condition.clone();
            let outbound_rpc_scheduler_clone = self.outbound_rpc_scheduler.clone();
            self.kv_proc_runtime.spawn(async move {Self::priority_handler(state_view_clone.clone(),
                                                      kv_tx_clone.clone(),
                                                      kv_unprocessed_clone.clone(),
                                                      recv_condition_clone.clone(),
                                                      outbound_rpc_scheduler_clone).await;
            });
        }

        while let Ok(message) = self.kv_rx.recv() {
            let curr_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
            let mut delta = 0.0;
            if curr_time > message.start_ms_since_epoch.unwrap() {
                delta = (curr_time - message.start_ms_since_epoch.unwrap()) as f64;
            }
            REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                .with_label_values(&["2_kv_req_coord_grpc_recv_spawning_req_handler"]).observe(delta);

            let _timer = REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "kv_requests_handler_timer"])
                .start_timer();

            let priority = message.seq_num.unwrap();

            kv_unprocessed.send(message, priority);
            REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "kv_req_pq_size"])
                .observe(kv_unprocessed.len() as f64);
        }
    }

    pub async fn priority_handler(state_view: Arc<RwLock<Option<Arc<S>>>>,
                            kv_tx: Arc<Vec<Vec<Arc<tokio::sync::Mutex<OutboundRpcHelper>>>>>,
                            pq: Arc<MpmcCpq<Message>>,
                            recv_condition: Arc<(tokio::sync::Mutex<bool>, Condvar)>,
                            outbound_rpc_scheduler: Arc<OutboundRpcScheduler>,) {
        let mut rng = StdRng::from_entropy();
        loop {
            let message = pq.recv().await;
            let state_view = state_view.clone();
            let kv_txs = kv_tx.clone();

            let outbound_rpc_scheduler_clone = outbound_rpc_scheduler.clone();
            let rand_send_thread_idx = rng.gen_range(0, kv_tx[0].len());
            Self::handle_message(message, state_view, kv_txs,rand_send_thread_idx, outbound_rpc_scheduler_clone);
        }
    }

    pub fn handle_message(
        message: Message,
        state_view: Arc<RwLock<Option<Arc<S>>>>,
        kv_tx: Arc<Vec<Vec<Arc<tokio::sync::Mutex<OutboundRpcHelper>>>>>,
        //rng: &mut StdRng,
        rand_send_thread_idx: usize,
        outbound_rpc_scheduler: Arc<OutboundRpcScheduler>,
    ) {
        let start_ms_since_epoch = message.start_ms_since_epoch.unwrap();
        {
            let curr_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let mut delta = 0.0;
            if curr_time > start_ms_since_epoch {
                delta = (curr_time - start_ms_since_epoch) as f64;
            }
            REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                .with_label_values(&["3_kv_req_coord_handler_st"])
                .observe(delta);
        }
        // we don't know the shard id until we deserialize the message, so lets default it to 0
        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests"])
            .start_timer();

        let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_req_deser"])
            .start_timer();
        let req: RemoteKVRequest = bcs::from_bytes(&message.data).unwrap();
        drop(bcs_deser_timer);

        let timer_2 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_2"])
            .start_timer();
        let (shard_id, state_keys) = req.into();
        trace!(
            "remote state view service - received request for shard {} with {} keys",
            shard_id,
            state_keys.len()
        );
        let state_view_read_lock = state_view.read().unwrap();
        let resp = state_keys
            .into_iter()
            .map(|state_key| {
                let state_value = if let Some(state_view_some) = state_view_read_lock.as_ref() {
                    state_view_some.get_state_value(&state_key).unwrap()
                }
                else {
                    None
                };
                (state_key, state_value)
            })
            .collect_vec();
        drop(state_view_read_lock);
        drop(timer_2);

        // let resp = state_keys
        //     .into_iter()
        //     .map(|state_key| {
        //         let state_value = state_view
        //             .read()
        //             .unwrap()
        //             .as_ref()
        //             .unwrap()
        //             .get_state_value(&state_key)
        //             .unwrap();
        //         (state_key, state_value)
        //     })
        //     .collect_vec();
        // drop(timer_2);

        let timer_3 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_3"])
            .start_timer();
        let len = resp.len();
        let resp = RemoteKVResponse::new(resp);
        let bcs_ser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_resp_ser"])
            .start_timer();
        let resp_serialized = bcs::to_bytes(&resp).unwrap();
        drop(bcs_ser_timer);
        trace!(
            "remote state view service - sending response for shard {} with {} keys",
            shard_id,
            len
        );
        let seq_num = message.seq_num.unwrap();
        drop(resp);
        drop(message);
        let resp_message = Message::create_with_metadata(resp_serialized, start_ms_since_epoch, seq_num, shard_id as u64);
        drop(timer_3);

        let _timer_4 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_4"])
            .start_timer();

        let timer_6 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_send"])
            .start_timer();
        let kv_tx_clone = kv_tx.clone();
        outbound_rpc_scheduler.send(resp_message,
                                    MessageType::new("remote_kv_response".to_string()),
                                    kv_tx_clone[shard_id][rand_send_thread_idx].clone(),
                                    seq_num + 2);
        drop(timer_6);
        {
            let curr_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let mut delta = 0.0;
            if curr_time > start_ms_since_epoch {
                delta = (curr_time - start_ms_since_epoch) as f64;
            }
            REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                .with_label_values(&["4_kv_req_coord_handler_end"])
                .observe(delta);
        }
    }
}
