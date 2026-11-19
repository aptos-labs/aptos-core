// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{RemoteKVRequest, RemoteKVResponse};
use aptos_secure_net::network_controller::{Message, NetworkController};
use crossbeam_channel::{Receiver, Sender};
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

extern crate itertools;
use crate::metrics::REMOTE_EXECUTOR_TIMER;
use aptos_logger::trace;
use aptos_types::state_store::{StateView, TStateView};
use itertools::Itertools;

pub struct RemoteStateViewService<S: StateView + Sync + Send + 'static> {
    kv_rx: Receiver<Message>,
    kv_tx: Arc<Vec<Sender<Message>>>,
    thread_pool: Arc<rayon::ThreadPool>,
    state_view: Arc<RwLock<Option<Arc<S>>>>,
}

impl<S: StateView + Sync + Send + 'static> RemoteStateViewService<S> {
    pub fn new(
        controller: &mut NetworkController,
        remote_shard_addresses: Vec<SocketAddr>,
        num_threads: Option<usize>,
    ) -> Self {
        let num_threads = num_threads.unwrap_or_else(num_cpus::get);
        let thread_pool = Arc::new(
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .unwrap(),
        );
        let kv_request_type = "remote_kv_request";
        let kv_response_type = "remote_kv_response";
        let result_rx = controller.create_inbound_channel(kv_request_type.to_string());
        let command_txs = remote_shard_addresses
            .iter()
            .map(|address| {
                controller.create_outbound_channel(*address, kv_response_type.to_string())
            })
            .collect_vec();
        Self {
            kv_rx: result_rx,
            kv_tx: Arc::new(command_txs),
            thread_pool,
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
            let state_view = self.state_view.clone();
            let kv_txs = self.kv_tx.clone();
            self.thread_pool.spawn(move || {
                Self::handle_message(message, state_view, kv_txs);
            });
        }
    }

    pub fn handle_message(
        message: Message,
        state_view: Arc<RwLock<Option<Arc<S>>>>,
        kv_tx: Arc<Vec<Sender<Message>>>,
    ) {
        // we don't know the shard id until we deserialize the message, so lets default it to 0
        let _timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_requests"])
            .start_timer();
        let bcs_deser_timer = REMOTE_EXECUTOR_TIMER
            .with_label_values(&["0", "kv_req_deser"])
            .start_timer();
        let req: RemoteKVRequest = bcs::from_bytes(&message.data).unwrap();
        drop(bcs_deser_timer);

        let (shard_id, state_keys, epoch) = req.into();

        // Take a single snapshot of the state view for the whole request. A straggler
        // request can arrive after the block it belongs to has finished and the state view
        // has been dropped; the shard discards responses from other epochs, so drop the
        // request instead of panicking.
        let Some(state_view) = state_view.read().unwrap().clone() else {
            trace!(
                "Dropping KV request for shard {} with {} keys, no state view set",
                shard_id,
                state_keys.len()
            );
            return;
        };
        trace!(
            "remote state view service - received request for shard {} with {} keys",
            shard_id,
            state_keys.len()
        );
        let resp = state_keys
            .into_iter()
            .map(|state_key| {
                let state_value = state_view.get_state_value(&state_key).unwrap();
                (state_key, state_value)
            })
            .collect_vec();
        let len = resp.len();
        let resp = RemoteKVResponse::new(resp, epoch);
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
        let message = Message::new(resp);
        kv_tx[shard_id].send(message).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RemoteKVRequest;
    use aptos_transaction_simulation::InMemoryStateStore;
    use aptos_types::state_store::state_key::StateKey;

    #[test]
    fn test_handle_message_without_state_view_is_noop() {
        // A straggler KV request can arrive after the block has finished and the state
        // view has been dropped; the request must be dropped instead of panicking.
        let request = RemoteKVRequest::new(0, 1, vec![StateKey::raw(b"key1")]);
        let message = Message::new(bcs::to_bytes(&request).unwrap());
        let state_view: Arc<RwLock<Option<Arc<InMemoryStateStore>>>> = Arc::new(RwLock::new(None));
        let (tx, rx) = crossbeam_channel::unbounded::<Message>();
        let kv_tx = Arc::new(vec![tx]);
        RemoteStateViewService::<InMemoryStateStore>::handle_message(message, state_view, kv_tx);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn test_handle_message_response_echoes_request_epoch() {
        let state_store = Arc::new(InMemoryStateStore::from_head_genesis());
        let state_view: Arc<RwLock<Option<Arc<InMemoryStateStore>>>> =
            Arc::new(RwLock::new(Some(state_store)));
        let request = RemoteKVRequest::new(0, 7, vec![StateKey::raw(b"key1")]);
        let message = Message::new(bcs::to_bytes(&request).unwrap());
        let (tx, rx) = crossbeam_channel::unbounded::<Message>();
        let kv_tx = Arc::new(vec![tx]);
        RemoteStateViewService::<InMemoryStateStore>::handle_message(message, state_view, kv_tx);
        let response: RemoteKVResponse = bcs::from_bytes(&rx.try_recv().unwrap().data).unwrap();
        assert_eq!(response.epoch, 7);
        assert_eq!(response.inner.len(), 1);
        assert_eq!(response.inner[0].0, StateKey::raw(b"key1"));
        assert!(response.inner[0].1.is_none());
    }
}
