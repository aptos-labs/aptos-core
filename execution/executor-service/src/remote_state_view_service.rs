// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use std::ops::DerefMut;
use std::time::SystemTime;

extern crate itertools;
use crate::metrics::REMOTE_EXECUTOR_TIMER;
use aptos_logger::{info, trace};
use aptos_types::state_store::{StateView, TStateView};
use itertools::Itertools;
use std::sync::Mutex;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER;

pub struct RemoteStateViewService<S: StateView + Sync + Send + 'static> {
    kv_rx: Receiver<Message>,
    kv_tx: Arc<Vec<Mutex<OutboundRpcHelper>>>,
    thread_pool: Arc<Vec<rayon::ThreadPool>>,
    state_view: Arc<RwLock<Option<Arc<S>>>>,
}

impl<S: StateView + Sync + Send + 'static> RemoteStateViewService<S> {
    pub fn new(
        controller: &mut NetworkController,
        remote_shard_addresses: Vec<SocketAddr>,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = num_threads.unwrap_or_else(num_cpus::get);
        info!("num threads for remote state view service: {}", num_threads);
        let mut thread_pool = vec![];
        for _ in 0..remote_shard_addresses.len() {
            thread_pool.push(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(num_threads)
                    .thread_name(|i| format!("remote-state-view-service-kv-request-handler-{}", i))
                    .build()
                    .unwrap(),
            );
        }
        let kv_request_type = "remote_kv_request";
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_request_type.to_string());
        let command_txs = remote_shard_addresses
            .iter()
            .map(|address| {
                //controller.create_outbound_channel(*address, kv_response_type.to_string())
                Mutex::new(OutboundRpcHelper::new(controller.get_self_addr(), *address))
            })
            .collect_vec();
        Self {
            kv_rx: result_rx,
            kv_tx: Arc::new(command_txs),
            thread_pool: Arc::new(thread_pool),
            state_view: Arc::new(RwLock::new(None)),
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
        while let Ok(message) = self.kv_rx.recv() {
            if message.start_ms_since_epoch.is_some() {
                let curr_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
                let mut delta = 0.0;
                if curr_time > message.start_ms_since_epoch.unwrap() {
                    delta = (curr_time - message.start_ms_since_epoch.unwrap()) as f64;
                }
                REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
                    .with_label_values(&["2_kv_req_coord_grpc_recv_spawning_req_handler"]).observe(delta);
            }
            let _timer = REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "kv_requests_handler_timer"])
                .start_timer();
            let state_view = self.state_view.clone();
            let kv_txs = self.kv_tx.clone();

            let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
                .with_label_values(&["0", "kv_req_deser"])
                .start_timer();
            let req: RemoteKVRequest = bcs::from_bytes(&message.data).unwrap();
            drop(bcs_deser_timer);

            let shard_id = req.shard_id;
            self.thread_pool[shard_id].spawn_fifo(move || {
                Self::handle_message(message, req, state_view, kv_txs);
            });
        }
    }

    pub fn handle_message(
        message: Message,
        req: RemoteKVRequest,
        state_view: Arc<RwLock<Option<Arc<S>>>>,
        kv_tx: Arc<Vec<Mutex<OutboundRpcHelper>>>,
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


        let (shard_id, state_keys) = req.into();
        trace!(
            "remote state view service - received request for shard {} with {} keys",
            shard_id,
            state_keys.len()
        );
        let resp = state_keys
            .into_iter()
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
            .collect_vec();
        let len = resp.len();
        let resp = RemoteKVResponse::new(resp);
        let bcs_ser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_resp_ser"])
            .start_timer();
        let resp = bcs::to_bytes(&resp).unwrap();
        drop(bcs_ser_timer);
        trace!(
            "remote state view service - sending response for shard {} with {} keys",
            shard_id,
            len
        );
        let message = Message::create_with_duration(resp, start_ms_since_epoch);
        kv_tx[shard_id].lock().unwrap().send(message, &MessageType::new("remote_kv_response".to_string()));
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
