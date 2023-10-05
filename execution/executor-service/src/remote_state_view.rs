// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, NetworkController};
use aptos_types::state_store::state_key::StateKey;
use aptos_vm::sharded_block_executor::remote_state_value::RemoteStateValue;
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
};
use std::time::Instant;

extern crate itertools;
use anyhow::Result;
use aptos_logger::{info, trace};
use aptos_state_view::TStateView;
use aptos_types::{
    block_executor::partitioner::ShardId,
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
};
use dashmap::DashMap;

pub static REMOTE_STATE_KEY_BATCH_SIZE: usize = 200;

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

static mut get_remote_val_time: f64 = 0.0;
static mut get_remote_val_count: usize = 0;
static mut get_prefetched_val_time_approx: f64 = 0.0;
static mut kv_response_process_time : f64 = 0.0;
static mut kv_response_process_count : usize = 0;
static mut kv_response_deser_time : f64 = 0.0;

pub struct RemoteStateViewClient {
    shard_id: ShardId,
    kv_tx: Arc<Sender<Message>>,
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
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .thread_name(move |index| format!("remote-state-view-shard-{}-{}", shard_id, index))
                .num_threads(num_cpus::get())
                .build()
                .unwrap(),
        );
        let kv_request_type = "remote_kv_request";
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_response_type.to_string());
        let command_tx =
            controller.create_outbound_channel(coordinator_address, kv_request_type.to_string());
        let state_view = Arc::new(RwLock::new(RemoteStateView::new()));
        let state_value_receiver = RemoteStateValueReceiver::new(
            shard_id,
            state_view.clone(),
            result_rx,
            thread_pool.clone(),
        );

        unsafe {
            get_remote_val_time = 0.0;
            get_remote_val_count = 0;
            get_prefetched_val_time_approx = 0.0;
            kv_response_process_time = 0.0;
            kv_response_process_count = 0;
            kv_response_deser_time = 0.0;
        }

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
        info!("&&&&&&&&&&&&& Prefetching {} state keys", state_keys.len());
        self.pre_fetch_state_values(state_keys);
    }

    fn pre_fetch_state_values(&self, state_keys: Vec<StateKey>) {
        state_keys.clone().into_iter().for_each(|state_key| {
            self.state_view.read().unwrap().insert_state_key(state_key);
        });
        state_keys
            .chunks(REMOTE_STATE_KEY_BATCH_SIZE)
            .map(|state_keys_chunk| state_keys_chunk.to_vec())
            .for_each(|state_keys| {
                let sender = self.kv_tx.clone();
                let shard_id = self.shard_id;
                self.thread_pool.spawn(move || {
                    Self::send_state_value_request(shard_id, sender, state_keys);
                });
            });
    }

    fn send_state_value_request(
        shard_id: ShardId,
        sender: Arc<Sender<Message>>,
        state_keys: Vec<StateKey>,
    ) {
        let request = RemoteKVRequest::new(shard_id, state_keys);
        let request_message = bcs::to_bytes(&request).unwrap();
        sender.send(Message::new(request_message)).unwrap();
    }

    pub fn print_info(&self) {
        unsafe {
            info!("&&&&&&&&&&&& Total get remote val time is {} for {} calls", get_remote_val_time, get_remote_val_count);
            info!("&&&&&&&&&&&& Total approx get prefetched val time is {}", get_prefetched_val_time_approx);
            info!("&&&&&&&&&&&& Total kv response process time is {} for {} calls", kv_response_process_time, kv_response_process_count);
            info!("&&&&&&&&&&&& Total kv response deser time is {}", kv_response_deser_time);
            get_remote_val_time = 0.0;
            get_remote_val_count = 0;
            get_prefetched_val_time_approx = 0.0;
            kv_response_process_time = 0.0;
            kv_response_process_count = 0;
            kv_response_deser_time = 0.0;
        }
    }
}

impl TStateView for RemoteStateViewClient {
    type Key = StateKey;

    fn get_state_value(&self, state_key: &StateKey) -> Result<Option<StateValue>> {
        let state_view_reader = self.state_view.read().unwrap();
        if state_view_reader.has_state_key(state_key) {
            // If the key is already in the cache then we return it.
            let start_time = Instant::now();
            let val = state_view_reader.get_state_value(state_key);
            unsafe {
                get_prefetched_val_time_approx += start_time.elapsed().as_secs_f64();
            }
            return val
        }
        // If the value is not already in the cache then we pre-fetch it and wait for it to arrive.
        let start_time = Instant::now();
        self.pre_fetch_state_values(vec![state_key.clone()]);
        let val = state_view_reader.get_state_value(state_key);
        unsafe {
            get_remote_val_time += start_time.elapsed().as_secs_f64();
            get_remote_val_count += 1;
        }
        val
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        unimplemented!("get_usage is not implemented for RemoteStateView")
    }
}

struct RemoteStateValueReceiver {
    shard_id: ShardId,
    state_view: Arc<RwLock<RemoteStateView>>,
    kv_rx: Receiver<Message>,
    thread_pool: Arc<rayon::ThreadPool>,
}

impl RemoteStateValueReceiver {
    fn new(
        shard_id: ShardId,
        state_view: Arc<RwLock<RemoteStateView>>,
        kv_rx: Receiver<Message>,
        thread_pool: Arc<rayon::ThreadPool>,
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
            let start_time = Instant::now();
            let state_view = self.state_view.clone();
            let shard_id = self.shard_id;
            self.thread_pool.spawn(move || {
                Self::handle_message(shard_id, message, state_view);
            });
        }
    }

    fn handle_message(
        shard_id: ShardId,
        message: Message,
        state_view: Arc<RwLock<RemoteStateView>>,
    ) {
        let start_time = Instant::now();
        let response: RemoteKVResponse = bcs::from_bytes(&message.data).unwrap();
        unsafe {
            kv_response_deser_time += start_time.elapsed().as_secs_f64();
        }
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
        unsafe {
            kv_response_process_time += start_time.elapsed().as_secs_f64();
            kv_response_process_count += 1;
        }
    }
}
