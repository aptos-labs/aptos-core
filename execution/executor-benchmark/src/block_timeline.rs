// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A per-block, per-stage trace of the pipeline: when each stage picked a
//! block up, when it was done, and how the executor's time inside a block
//! split. Written as JSON lines to `BLOCK_TIMELINE_PATH` when set, so the
//! gaps between blocks can be attributed after the run.

use aptos_executor::metrics::OTHER_TIMERS;
use aptos_infallible::Mutex;
use once_cell::sync::Lazy;
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufWriter, Write},
    time::Instant,
};

static SINK: Lazy<Option<Mutex<BufWriter<File>>>> = Lazy::new(|| {
    let path = std::env::var_os("BLOCK_TIMELINE_PATH")?;
    File::create(path)
        .ok()
        .map(|f| Mutex::new(BufWriter::new(f)))
});

/// The executor's named sub-timers that are read before and after each block.
const EXECUTE_TIMERS: [&str; 4] = [
    "get_state_view",
    "vm_execute_block",
    "parse_raw_output",
    "parse_raw_output__to_commit",
];

pub(crate) fn enabled() -> bool {
    SINK.is_some()
}

fn now_secs() -> f64 {
    static START: Lazy<Instant> = Lazy::new(Instant::now);
    START.elapsed().as_secs_f64()
}

fn timer_sums() -> BTreeMap<&'static str, f64> {
    EXECUTE_TIMERS
        .iter()
        .map(|name| {
            (
                *name,
                OTHER_TIMERS.with_label_values(&[name]).get_sample_sum(),
            )
        })
        .collect()
}

/// A stage's work on one block, from pick-up to hand-off.
pub(crate) struct StageSpan {
    stage: &'static str,
    block: usize,
    num_txns: usize,
    start: f64,
    timers_before: Option<BTreeMap<&'static str, f64>>,
}

impl StageSpan {
    pub fn start(stage: &'static str, block: usize, num_txns: usize) -> Self {
        let timers_before = (stage == "execute" && enabled()).then(timer_sums);
        Self {
            stage,
            block,
            num_txns,
            start: now_secs(),
            timers_before,
        }
    }

    pub fn finish(self) {
        let Some(sink) = SINK.as_ref() else {
            return;
        };
        let end = now_secs();
        let mut line = format!(
            "{{\"stage\":\"{}\",\"block\":{},\"num_txns\":{},\"start\":{:.6},\"end\":{:.6}",
            self.stage, self.block, self.num_txns, self.start, end
        );
        if let Some(before) = self.timers_before {
            for (name, after) in timer_sums() {
                line.push_str(&format!(",\"{}\":{:.6}", name, after - before[name]));
            }
        }
        line.push_str("}\n");
        let _ = sink.lock().write_all(line.as_bytes());
    }
}

pub(crate) fn flush() {
    if let Some(sink) = SINK.as_ref() {
        let _ = sink.lock().flush();
    }
}
