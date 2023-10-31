// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, MessageType, NetworkController};
use aptos_types::state_store::state_key::StateKey;
use aptos_vm::sharded_block_executor::remote_state_value::RemoteStateValue;
use crossbeam_channel::Receiver;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
};
use std::sync::Mutex;
use std::time::SystemTime;

extern crate itertools;
use crate::metrics::{REMOTE_EXECUTOR_REMOTE_KV_COUNT, REMOTE_EXECUTOR_TIMER};
use aptos_logger::trace;
use aptos_types::{
    block_executor::partitioner::ShardId,
    state_store::{
        state_storage_usage::StateStorageUsage, state_value::StateValue, Result, TStateView,
    },
};
use dashmap::DashMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rayon::ThreadPool;
use aptos_secure_net::grpc_network_service::outbound_rpc_helper::OutboundRpcHelper;
use aptos_secure_net::network_controller::metrics::REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER;

pub static REMOTE_STATE_KEY_BATCH_SIZE: usize = 200;
pub static REMOTE_KV_REQUEST_MSG_TYPE: &str = "remote_kv_request";

pub struct RemoteStateView {
    state_values: DashMap<StateKey, RemoteStateValue>,
}

impl RemoteStateView {
    pub fn new() -> Self {
        Self {
            state_values: DashMap::new(),
        }
    }

    pub fn has_state_key(&self, state_key: &StateKey) -> bool {
        self.state_values.contains_key(state_key)
    }

    pub fn set_state_value(&self, state_key: &StateKey, state_value: Option<StateValue>) {
        self.state_values
            .get(state_key)
            .unwrap()
            .set_value(state_value);
    }

    pub fn insert_state_key(&self, state_key: StateKey) {
        self.state_values
            .entry(state_key)
            .or_insert(RemoteStateValue::waiting());
    }

    pub fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        if let Some(value) = self.state_values.get(state_key) {
            let value_clone = value.clone();
            // It is possible that the value is not ready yet and the get_value call blocks. In that
            // case we explicitly drop the value to relinquish the read lock on the value. Cloning the
            // value should be in expensive as this is just cloning the underlying Arc.
            drop(value);
            return Ok(value_clone.get_value());
        }
        Ok(None)
    }
}

pub struct RemoteStateViewClient {
    shard_id: ShardId,
    kv_tx: Arc<Vec<Mutex<OutboundRpcHelper>>>,
    state_view: Arc<RwLock<RemoteStateView>>,
    thread_pool: Arc<rayon::ThreadPool>,
    _join_handle: Option<thread::JoinHandle<()>>,
}

impl RemoteStateViewClient {
    pub fn new(
        shard_id: ShardId,
        controller: &mut NetworkController,
        coordinator_address: SocketAddr,
    ) -> Self {
        let num_kv_req_threads = num_cpus::get() / 2;
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index| format!("remote-state-view-shard-send-request-{}-{}", shard_id, index))
                .num_threads(num_kv_req_threads)
                .build()
                .unwrap(),
        );
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_response_type.to_string());
        let mut command_tx = vec![];
        for _ in 0..num_kv_req_threads {
            command_tx.push(Mutex::new(OutboundRpcHelper::new(controller.get_self_addr(), coordinator_address)));
        }
        let state_view = Arc::new(RwLock::new(RemoteStateView::new()));
        let state_value_receiver = RemoteStateValueReceiver::new(
            shard_id,
            state_view.clone(),
            result_rx,
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index| format!("remote-state-view-shard-recv-resp-{}-{}", shard_id, index))
                .num_threads(num_cpus::get() / 2)
                .build()
                .unwrap(),
        );

        let join_handle = thread::Builder::new()
            .name(format!("remote-kv-receiver-{}", shard_id))
            .spawn(move || state_value_receiver.start())
            .unwrap();

        Self {
            shard_id,
            kv_tx: Arc::new(command_tx),
            state_view,
            thread_pool,
            _join_handle: Some(join_handle),
        }
    }

    pub fn init_for_block(&self, state_keys: Vec<StateKey>) {
        *self.state_view.write().unwrap() = RemoteStateView::new();
        REMOTE_EXECUTOR_REMOTE_KV_COUNT
            .with_label_values(&[&self.shard_id.to_string(), "prefetch_kv"])
            .inc_by(state_keys.len() as u64);
        self.pre_fetch_state_values(state_keys, false);
    }

    fn insert_keys_and_fetch_values(
        state_view_clone: Arc<RwLock<RemoteStateView>>,
        thread_pool: Arc<ThreadPool>,
        kv_tx: Arc<Vec<Mutex<OutboundRpcHelper>>>,
        shard_id: ShardId,
        state_keys: Vec<StateKey>,
    ) {
        state_keys.clone().into_iter().for_each(|state_key| {
            state_view_clone.read().unwrap().insert_state_key(state_key);
        });
        let mut seq_num = 0;

        state_keys
            .chunks(REMOTE_STATE_KEY_BATCH_SIZE)
            .map(|state_keys_chunk| state_keys_chunk.to_vec())
            .for_each(|state_keys| {
                let sender = kv_tx.clone();
                if state_keys.len() > 1 {
                    seq_num += 1;
                }
                thread_pool.spawn_fifo(move || {
                    let mut rng = StdRng::from_entropy();
                    let rand_send_thread_idx = rng.gen_range(0, sender.len());
                    Self::send_state_value_request(shard_id, sender, state_keys, rand_send_thread_idx, seq_num);
                });
            });
    }

    fn pre_fetch_state_values(&self, state_keys: Vec<StateKey>, sync_insert_keys: bool) {
        let state_view_clone = self.state_view.clone();
        let thread_pool_clone = self.thread_pool.clone();
        let kv_tx_clone = self.kv_tx.clone();
        let shard_id = self.shard_id;

        let insert_and_fetch = move || {
            Self::insert_keys_and_fetch_values(
                state_view_clone,
                thread_pool_clone,
                kv_tx_clone,
                shard_id,
                state_keys,
            );
        };
        if sync_insert_keys {
            // we want to insert keys synchronously here because when called from get_state_value()
            // it expects the key to be in the table while waiting for the value to be fetched from
            // remote state view.
            insert_and_fetch();
        } else {
            self.thread_pool.spawn_fifo(insert_and_fetch);
        }
    }

    fn send_state_value_request(
        shard_id: ShardId,
        sender: Arc<Vec<Mutex<OutboundRpcHelper>>>,
        state_keys: Vec<StateKey>,
        rand_send_thread_idx: usize,
        seq_num: u64,
    ) {
        let request = RemoteKVRequest::new(shard_id, state_keys);
        let request_message = bcs::to_bytes(&request).unwrap();
        let duration_since_epoch = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap().as_millis() as u64;

        let mut sender_lk = sender[rand_send_thread_idx].lock().unwrap();
        let curr_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis() as u64;
        let mut delta = 0.0;
        if curr_time > duration_since_epoch {
            delta = (curr_time - duration_since_epoch) as f64;
        }
        REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER
            .with_label_values(&["0_kv_req_grpc_shard_send_1_lock_acquired"]).observe(delta);
        sender_lk.send(Message::create_with_metadata(request_message, duration_since_epoch, seq_num, shard_id as u64),
                       &MessageType::new(REMOTE_KV_REQUEST_MSG_TYPE.to_string()));
    }
}

impl TStateView for RemoteStateViewClient {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        let state_view_reader = self.state_view.read().unwrap();
        if state_view_reader.has_state_key(state_key) {
            // If the key is already in the cache then we return it.
            return state_view_reader.get_state_value(state_key);
        }
        // If the value is not already in the cache then we pre-fetch it and wait for it to arrive.
        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&[&self.shard_id.to_string(), "non_prefetch_wait"])
            .start_timer();
        REMOTE_EXECUTOR_REMOTE_KV_COUNT
            .with_label_values(&[&self.shard_id.to_string(), "non_prefetch_kv"])
            .inc();
        self.pre_fetch_state_values(vec![state_key.clone()], true);
        state_view_reader.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        unimplemented!("get_usage is not implemented for RemoteStateView")
    }
}

struct RemoteStateValueReceiver {
    shard_id: ShardId,
    state_view: Arc<RwLock<RemoteStateView>>,
    kv_rx: Receiver<Message>,
    thread_pool: rayon::ThreadPool,
}

impl RemoteStateValueReceiver {
    fn new(
        shard_id: ShardId,
        state_view: Arc<RwLock<RemoteStateView>>,
        kv_rx: Receiver<Message>,
        thread_pool: rayon::ThreadPool,
    ) -> Self {
        Self {
            shard_id,
            state_view,
            kv_rx,
            thread_pool,
        }
    }

    fn start(&self) {
        while let Ok(message) = self.kv_rx.recv() {
            let state_view = self.state_view.clone();
            let shard_id = self.shard_id;
            self.thread_pool.spawn_fifo(move || {
                Self::handle_message(shard_id, message, state_view);
            });
        }
    }

    fn handle_message(
        shard_id: ShardId,
        message: Message,
        state_view: Arc<RwLock<RemoteStateView>>,
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
                .with_label_values(&["7_kv_resp_shard_handler_st"])
                .observe(delta);
        }

        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&[&shard_id.to_string(), "kv_responses"])
            .start_timer();
        let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&[&shard_id.to_string(), "kv_resp_deser"])
            .start_timer();
        let response: RemoteKVResponse = bcs::from_bytes(&message.data).unwrap();
        drop(bcs_deser_timer);

        REMOTE_EXECUTOR_REMOTE_KV_COUNT
            .with_label_values(&[&shard_id.to_string(), "kv_responses"])
            .inc();
        let state_view_lock = state_view.read().unwrap();
        trace!(
            "Received state values for shard {} with size {}",
            shard_id,
            response.inner.len()
        );
        response
            .inner
            .into_iter()
            .for_each(|(state_key, state_value)| {
                state_view_lock.set_state_value(&state_key, state_value);
            });

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
                .with_label_values(&["8_kv_resp_shard_handler_end"])
                .observe(delta);
        }
    }
}
