// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec, IntGaugeVecHelper};
use jemalloc_ctl::Error;
use once_cell::sync::Lazy;
use std::{sync::OnceLock, thread, time::Duration};

const COLLECTION_INTERVAL: Duration = Duration::from_secs(30);

/// Merged arena index that aggregates stats across all arenas.
/// This is `MALLCTL_ARENAS_ALL` from jemalloc's public API.
const ARENAS_ALL: usize = 4096;

static JEMALLOC_STATS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("aptos_jemalloc_stats", "jemalloc allocator statistics", &[
        "stat"
    ])
    .unwrap()
});

/// Spawns a background thread that periodically collects jemalloc statistics
/// and exports them as Prometheus gauges.
pub fn start_jemalloc_metrics_thread() {
    thread::Builder::new()
        .name("jemalloc-stats".into())
        .spawn(|| loop {
            thread::sleep(COLLECTION_INTERVAL);
            if let Err(e) = collect_once() {
                aptos_logger::warn!("jemalloc-stats: collection error: {}", e);
            }
        })
        .expect("failed to spawn jemalloc-stats thread");
}

fn page_size() -> usize {
    static PAGE_SIZE: OnceLock<usize> = OnceLock::new();
    *PAGE_SIZE.get_or_init(|| {
        let key = b"arenas.page\0";
        unsafe { jemalloc_ctl::raw::read(key) }.expect("failed to read arenas.page")
    })
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
        JEMALLOC_STATS.set_with(&[name], val as i64);
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
        let page_size = page_size();
        gauge("dirty", read_arena_usize("pdirty")? * page_size);
        gauge("muzzy", read_arena_usize("pmuzzy")? * page_size);
        gauge("tcache", read_arena_usize("tcache_bytes")?);
    }

    // Metadata THP: number of transparent huge pages backing jemalloc metadata.
    let n_thp: usize = unsafe { jemalloc_ctl::raw::read(b"stats.metadata_thp\0") }?;
    gauge("metadata_thp", n_thp);

    Ok(())
}
