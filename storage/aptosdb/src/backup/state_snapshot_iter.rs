// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::StateStore;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_storage_interface::{db_ensure, AptosDbError, Result as DbResult};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::{
    collections::VecDeque,
    sync::{
        mpsc::{Receiver, Sender, SyncSender},
        Arc,
    },
};

pub fn state_snapshot_iter(
    state_store: Arc<StateStore>,
    version: Version,
) -> impl Iterator<Item = DbResult<(StateKey, StateValue)>> + Send {
    // Channel must be bounded, otherwis memory usage can grow unbounded if the consuming side is slow.
    let (result_tx, result_rx) = std::sync::mpsc::sync_channel(0);

    // spawn and forget, error propagates through channel
    let _scheduler = std::thread::spawn(move || scheduler_thread(state_store, version, result_tx));

    result_rx.into_iter()
}

fn scheduler_thread(
    store: Arc<StateStore>,
    version: Version,
    result_tx: SyncSender<DbResult<(StateKey, StateValue)>>,
) {
    if let Err(err) = scheduler_thread_inner(store, version, &result_tx) {
        result_tx.send(Err(err)).ok();
    }
}

fn scheduler_thread_inner(
    store: Arc<StateStore>,
    version: Version,
    result_tx: &SyncSender<DbResult<(StateKey, StateValue)>>,
) -> DbResult<()> {
    const CONCURRENCY: usize = 2;
    const CHUNK_SIZE: usize = 100_000;

    let pool = THREAD_MANAGER.get_background_pool();

    let count = store.get_value_count(version)?;

    let mut sequencer = VecDeque::new();
    let mut start_idx = 0;

    while start_idx < count {
        let (tx, rx) = std::sync::mpsc::channel();
        sequencer.push_back(rx);

        let store = store.clone();
        let chunk_size = CHUNK_SIZE.min(count - start_idx);
        pool.spawn(move || iter_chunk(store, version, start_idx, chunk_size, tx));

        if sequencer.len() >= CONCURRENCY {
            let rx = sequencer.pop_front().unwrap();
            try_passthrough(rx, result_tx)?;
        }

        start_idx += CHUNK_SIZE;
    }

    // Drain all scheduled tasks.
    while let Some(rx) = sequencer.pop_front() {
        try_passthrough(rx, result_tx)?;
    }

    Ok(())
}

fn try_passthrough(
    upstream: Receiver<DbResult<(StateKey, StateValue)>>,
    downstream: &SyncSender<DbResult<(StateKey, StateValue)>>,
) -> DbResult<()> {
    while let Ok(res) = upstream.recv() {
        let record = res?;
        downstream
            .send(Ok(record))
            .map_err(|err| AptosDbError::Other(format!("Send Error: {}", err)))?;
    }
    Ok(())
}

fn iter_chunk(
    store: Arc<StateStore>,
    version: Version,
    start_idx: usize,
    num_items: usize,
    tx: Sender<DbResult<(StateKey, StateValue)>>,
) {
    if let Err(err) = iter_chunk_inner(store, version, start_idx, num_items, &tx) {
        tx.send(Err(err)).ok();
    }
}

fn iter_chunk_inner(
    store: Arc<StateStore>,
    version: Version,
    start_idx: usize,
    num_items: usize,
    tx: &Sender<DbResult<(StateKey, StateValue)>>,
) -> DbResult<()> {
    let iter = store.get_state_key_and_value_iter(version, start_idx)?;

    let mut total = 0;
    for record_res in iter {
        let record = record_res?;

        tx.send(Ok(record))
            .map_err(|err| AptosDbError::Other(format!("Send error: {}", err)))?;
        total += 1;
    }

    db_ensure!(
        total == num_items,
        "Expected {} items, got {}",
        num_items,
        total
    );

    Ok(())
}
