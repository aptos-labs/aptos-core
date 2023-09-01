// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use crate::{common::BASE_VERSION, metrics::TIMER, scheduler::PtxSchedulerClient};
use aptos_logger::trace;
use aptos_metrics_core::TimerHelper;
use aptos_state_view::StateView;
use aptos_types::state_store::state_key::StateKey;
use once_cell::sync::Lazy;
use rayon::Scope;
use std::sync::mpsc::{channel, Receiver, Sender};

const NUM_IO_THREADS: usize = 64;

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
        scheduler: PtxSchedulerClient,
        state_view: &'view (impl StateView + Sync),
    ) -> PtxStateReaderClient {
        let (work_tx, work_rx) = channel();
        scope.spawn(|_scope| {
            let _timer = TIMER.timer_with(&["state_reader_block_total"]);
            Self::work(work_rx, scheduler, state_view)
        });
        PtxStateReaderClient { work_tx }
    }

    fn work(
        work_rx: Receiver<Command>,
        scheduler: PtxSchedulerClient,
        state_view: &(impl StateView + Sync),
    ) {
        IO_POOL.scope(move |io_scope| loop {
            let scheduler = scheduler.clone();
            match work_rx.recv().expect("Channel closed.") {
                Command::Read { state_key } => io_scope.spawn(move |_io_scope| {
                    let value = state_view.get_state_value(&state_key).unwrap();
                    scheduler.inform_state_value((state_key, BASE_VERSION), value);
                }),
                Command::FinishBlock => {
                    trace!("finish_block.");
                    break;
                },
            }
        });
        trace!("IO scope exit.");
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
