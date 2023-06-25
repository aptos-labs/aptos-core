// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{common::BASE_VERSION, executor::PtxExecutorClient};
use aptos_state_view::StateView;
use aptos_types::state_store::state_key::StateKey;
use once_cell::sync::Lazy;
use rayon::Scope;
use std::sync::mpsc::{channel, Receiver, Sender};

const NUM_IO_THREADS: usize = 32;

pub(crate) static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(NUM_IO_THREADS)
        .thread_name(|index| format!("ptx_state_reader_io_{}", index))
        .build()
        .unwrap()
});

pub(crate) struct PtxStateReader;

impl PtxStateReader {
    pub fn spawn<'scope, 'view: 'scope>(
        scope: &Scope<'scope>,
        executor: PtxExecutorClient,
        state_view: &'view (impl StateView + Sync),
    ) -> PtxStateReaderClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(|_scope| Self::work(work_rx, executor, state_view));
        PtxStateReaderClient { work_tx }
    }

    fn work(
        work_rx: Receiver<Command>,
        executor: PtxExecutorClient,
        state_view: &(impl StateView + Sync),
    ) {
        IO_POOL.scope(move |io_scope| loop {
            let executor = executor.clone();
            match work_rx.recv().expect("Channel closed.") {
                Command::Read { state_key } => io_scope.spawn(move |_scope| {
                    let value = state_view.get_state_value(&state_key).unwrap();
                    executor.inform_state_value((state_key, BASE_VERSION), value);
                }),
                Command::FinishBlock => {
                    break;
                },
            }
        });
    }
}

#[derive(Clone)]
pub(crate) struct PtxStateReaderClient {
    work_tx: Sender<Command>,
}

impl PtxStateReaderClient {
    pub fn schedule_read(&self, state_key: StateKey) {
        self.send_to_worker(Command::Read { state_key })
    }

    pub fn finish_block(&self) {
        self.send_to_worker(Command::FinishBlock)
    }

    fn send_to_worker(&self, command: Command) {
        self.work_tx.send(command).expect("Work thread died.");
    }
}

enum Command {
    Read { state_key: StateKey },
    FinishBlock,
}
