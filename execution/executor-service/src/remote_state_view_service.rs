// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
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
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator};
use rayon::iter::ParallelIterator;
use tokio::runtime;
use tokio::runtime::Runtime;
use aptos_drop_helper::DEFAULT_DROPPER;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER;

pub struct RemoteStateViewService<S: StateView + Sync + Send + 'static> {
    kv_rx: Receiver<Message>,
    kv_unprocessed_pq: Arc<ConcurrentPriorityQueue<Message, u64>>,
    kv_tx: Arc<Vec<Vec<tokio::sync::Mutex<OutboundRpcHelper>>>>,
    //thread_pool: Arc<Vec<rayon::ThreadPool>>,
    // thread_pool for processing kv requests
    thread_pool: Arc<rayon::ThreadPool>,
    state_view: Arc<RwLock<Option<Arc<S>>>>,
    recv_condition: Arc<(Mutex<bool>, Condvar)>,
    outbound_rpc_runtime: Arc<Runtime>,
    //rt_kv_proc_runtime: Arc<Runtime>,
}

impl<S: StateView + Sync + Send + 'static> RemoteStateViewService<S> {
    pub fn new(
        controller: &mut NetworkController,
        remote_shard_addresses: Vec<SocketAddr>,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = 60;//remote_shard_addresses.len() * 2; //num_threads.unwrap_or_else(num_cpus::get);
        let num_kv_req_threads = 16; //= num_cpus::get() / 2;
        let num_shards = remote_shard_addresses.len();
        info!("num threads for remote state view service: {}", num_threads);
        /*let mut thread_pool = vec![];
        for _ in 0..remote_shard_addresses.len() {
            thread_pool.push(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .thread_name(|i| format!("remote-state-view-service-kv-request-handler-{}", i))
                    .build()
                    .unwrap(),
            );
        }*/
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .thread_name(|i| format!("remote-state-view-service-kv-request-handler-{}", i))
            .build()
            .unwrap();
        let kv_request_type = "remote_kv_request";
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_request_type.to_string());
        let command_txs = remote_shard_addresses
            .iter()
            .map(|address| {
                //controller.create_outbound_channel(*address, kv_response_type.to_string())
                let mut command_tx = vec![];
                for _ in 0..num_kv_req_threads {
                    command_tx.push(tokio::sync::Mutex::new(OutboundRpcHelper::new(controller.get_self_addr(), *address, controller.get_outbound_rpc_runtime())));
                }
                command_tx
            })
            .collect_vec();
        Self {
            kv_rx: result_rx,
            kv_unprocessed_pq: Arc::new(ConcurrentPriorityQueue::new()),
            kv_tx: Arc::new(command_txs),
            thread_pool: Arc::new(thread_pool),
            state_view: Arc::new(RwLock::new(None)),
            recv_condition: Arc::new((Mutex::new(false), Condvar::new())),
            outbound_rpc_runtime: controller.get_outbound_rpc_runtime(),
            //rt_kv_proc_runtime: Arc::new(rt_kv_proc),
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
        //let (signal_tx, signal_rx) = unbounded();
        let thread_pool_clone = self.thread_pool.clone();
        let num_handlers = 120;
        info!("Num handlers created is {}", num_handlers);
        for i in 0..num_handlers {
            let state_view_clone = self.state_view.clone();
            let kv_tx_clone = self.kv_tx.clone();
            let kv_unprocessed_pq_clone = self.kv_unprocessed_pq.clone();
            let recv_condition_clone = self.recv_condition.clone();
            let outbound_rpc_runtime_clone = self.outbound_rpc_runtime.clone();
            thread_pool_clone.spawn(move || {
                Self::priority_handler(state_view_clone.clone(),
                                       kv_tx_clone.clone(),
                                       kv_unprocessed_pq_clone,
                                        recv_condition_clone.clone(),
                                       outbound_rpc_runtime_clone)
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
            {
                let (lock, cvar) = &*self.recv_condition;
                let _lg = lock.lock().unwrap();
                self.kv_unprocessed_pq.push(message, priority);
                self.recv_condition.1.notify_all();
            }
            REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "kv_req_pq_size"])
                .observe(self.kv_unprocessed_pq.len() as f64);
        }
    }

    // pub fn priority_handler(state_view: Arc<RwLock<Option<Arc<S>>>>,
    //                         kv_tx: Arc<Vec<Vec<tokio::sync::Mutex<OutboundRpcHelper>>>>,
    //                         kv_int_rx: Arc<Receiver<Message>>,
    //                         outbound_rpc_runtime: Arc<Runtime>) {
    //     let mut rng = StdRng::from_entropy();
    //     let mut curr_time;
    //     let mut prev_time = SystemTime::now()
    //         .duration_since(SystemTime::UNIX_EPOCH)
    //         .unwrap()
    //         .as_millis() as u64;
    //     while let Ok(message) = kv_int_rx.recv() {
    //         curr_time = SystemTime::now()
    //             .duration_since(SystemTime::UNIX_EPOCH)
    //             .unwrap()
    //             .as_millis() as u64;
    //         REMOTE_EXECUTOR_TIMER
    //             .with_label_values(&["0", "kv_proc_thread_waiting_time"])
    //             .observe(((curr_time - prev_time) / 1000) as f64);
    //
    //         let state_view = state_view.clone();
    //         let kv_txs = kv_tx.clone();
    //
    //         let outbound_rpc_runtime_clone = outbound_rpc_runtime.clone();
    //         Self::handle_message(message, state_view, kv_txs, rng.gen_range(0, kv_tx[0].len()), outbound_rpc_runtime_clone);
    //
    //         prev_time = SystemTime::now()
    //         .duration_since(SystemTime::UNIX_EPOCH)
    //         .unwrap()
    //         .as_millis() as u64;
    //     }
    // }

    pub fn priority_handler(state_view: Arc<RwLock<Option<Arc<S>>>>,
                            kv_tx: Arc<Vec<Vec<tokio::sync::Mutex<OutboundRpcHelper>>>>,
                            pq: Arc<ConcurrentPriorityQueue<Message, u64>>,
                            recv_condition: Arc<(Mutex<bool>, Condvar)>,
                            outbound_rpc_runtime: Arc<Runtime>) {
        let mut rng = StdRng::from_entropy();
        let mut curr_time;
            let mut prev_time = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
        loop {

            let (lock, cvar) = &*recv_condition;
            let mut lg = lock.lock().unwrap();
            if pq.is_empty() {
                lg = cvar.wait(lg).unwrap();
            }
            drop(lg);
            curr_time = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    REMOTE_EXECUTOR_TIMER
                        .with_label_values(&["0", "kv_proc_thread_waiting_time"])
                        .observe(((curr_time - prev_time) / 1000) as f64);

            if let Some(message) = pq.pop() {
                let state_view = state_view.clone();
                let kv_txs = kv_tx.clone();

                let outbound_rpc_runtime_clone = outbound_rpc_runtime.clone();
                Self::handle_message(message, state_view, kv_txs, rng.gen_range(0, kv_tx[0].len()), outbound_rpc_runtime_clone);

                prev_time = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
            }
        }
    }

    pub fn handle_message(
        message: Message,
        state_view: Arc<RwLock<Option<Arc<S>>>>,
        kv_tx: Arc<Vec<Vec<tokio::sync::Mutex<OutboundRpcHelper>>>>,
        //rng: &mut StdRng,
        rand_send_thread_idx: usize,
        outbound_rpc_runtime: Arc<Runtime>,
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
        let mut resp = vec![]; 
        state_keys
        .into_par_iter()
        .map(|state_key| {
            let state_value = state_view
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .get_state_value(&state_key)
                .unwrap();
            (state_key, state_value)
        })
        .collect_into_vec(&mut resp);
        drop(timer_2);

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
        //DEFAULT_DROPPER.schedule_drop(resp);
        //DEFAULT_DROPPER.schedule_drop(message);
        // info!("Processing message with seq_num: {}", seq_num);
        let resp_message = Message::create_with_metadata(resp_serialized, start_ms_since_epoch, seq_num, shard_id as u64);
        drop(timer_3);

        let _timer_4 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_4"])
            .start_timer();

        // let rand_send_thread_idx = rng.gen_range(0, kv_tx[shard_id].len());
        let kv_tx_clone = kv_tx.clone();
        let timer_6 = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests_send"])
            .start_timer();
        outbound_rpc_runtime.spawn(async move {
            kv_tx_clone[shard_id][rand_send_thread_idx].lock().await.send_async(resp_message, &MessageType::new("remote_kv_response".to_string())).await;
        });
        //kv_tx_clone[shard_id][rand_send_thread_idx].lock().unwrap().send(resp_message, &MessageType::new("remote_kv_response".to_string()));
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