// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use jemalloc_ctl::Error;
use once_cell::sync::Lazy;
use std::{thread, time::Duration};

const COLLECTION_INTERVAL: Duration = Duration::from_secs(30);
const PAGE_SIZE: usize = 4096;

/// Merged arena index that aggregates stats across all arenas.
/// This is `MALLCTL_ARENAS_ALL` from jemalloc's public API.
const ARENAS_ALL: usize = 4096;

static JEMALLOC_BYTES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_jemalloc_bytes",
        "jemalloc allocator statistics in bytes",
        &["stat"]
    )
    .unwrap()
});

/// Spawns a background thread that periodically collects jemalloc statistics
/// and exports them as Prometheus gauges.
pub fn start_jemalloc_metrics_thread() {
    thread::Builder::new()
        .name("jemalloc-stats".into())
        .spawn(metrics_loop)
        .expect("failed to spawn jemalloc-stats thread");
}

fn metrics_loop() {
    loop {
        thread::sleep(COLLECTION_INTERVAL);

        if let Err(e) = collect_once() {
            aptos_logger::warn!("jemalloc-stats: collection error: {}", e);
        }
    }
}

/// Reads a `usize` from a mallctl key under the merged arena namespace.
unsafe fn read_arena_usize(name: &str) -> Result<usize, Error> {
    let key = format!("stats.arenas.{ARENAS_ALL}.{name}\0");
    unsafe { jemalloc_ctl::raw::read(key.as_bytes()) }
}

fn collect_once() -> Result<(), Error> {
    // Advance the epoch so subsequent reads return fresh values.
    jemalloc_ctl::epoch::advance()?;

    let gauge = |name: &str, val: usize| {
        JEMALLOC_BYTES.with_label_values(&[name]).set(val as i64);
    };

    // Global stats (bytes).
    gauge("allocated", jemalloc_ctl::stats::allocated::read()?);
    gauge("active", jemalloc_ctl::stats::active::read()?);
    gauge("metadata", jemalloc_ctl::stats::metadata::read()?);
    gauge("resident", jemalloc_ctl::stats::resident::read()?);
    gauge("mapped", jemalloc_ctl::stats::mapped::read()?);
    gauge("retained", jemalloc_ctl::stats::retained::read()?);

    // Per-arena stats aggregated across all arenas (raw mallctl).
    unsafe {
        gauge("dirty", read_arena_usize("pdirty")? * PAGE_SIZE);
        gauge("muzzy", read_arena_usize("pmuzzy")? * PAGE_SIZE);
        gauge("tcache", read_arena_usize("tcache_bytes")?);
    }

    Ok(())
}
