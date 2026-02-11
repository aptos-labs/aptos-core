// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use jemalloc_ctl::Error;
use once_cell::sync::Lazy;
use std::{thread, time::Duration};

const COLLECTION_INTERVAL: Duration = Duration::from_secs(30);

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
    // Force metric registration so the gauge exists before the first scrape.
    Lazy::force(&JEMALLOC_BYTES);

    let page_size = match read_page_size() {
        Ok(ps) => ps,
        Err(e) => {
            aptos_logger::warn!(
                "jemalloc-stats: failed to read page size, defaulting to 4096: {}",
                e
            );
            4096
        },
    };

    thread::Builder::new()
        .name("jemalloc-stats".into())
        .spawn(move || metrics_loop(page_size))
        .expect("failed to spawn jemalloc-stats thread");
}

fn read_page_size() -> Result<usize, Error> {
    // Safety: `arenas.page` returns a `size_t`.
    unsafe { jemalloc_ctl::raw::read(b"arenas.page\0") }
}

fn metrics_loop(page_size: usize) {
    loop {
        thread::sleep(COLLECTION_INTERVAL);

        if let Err(e) = collect_once(page_size) {
            aptos_logger::warn!("jemalloc-stats: collection error: {}", e);
        }
    }
}

/// Reads a `usize` from a mallctl key under the merged arena namespace.
unsafe fn read_arena_usize(name: &str) -> Result<usize, Error> {
    let key = format!("stats.arenas.{ARENAS_ALL}.{name}\0");
    unsafe { jemalloc_ctl::raw::read(key.as_bytes()) }
}

fn collect_once(page_size: usize) -> Result<(), Error> {
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
        gauge("dirty", read_arena_usize("pdirty")? * page_size);
        gauge("muzzy", read_arena_usize("pmuzzy")? * page_size);
        gauge("tcache", read_arena_usize("tcache_bytes")?);
    }

    Ok(())
}
