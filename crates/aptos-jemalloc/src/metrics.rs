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
                "jemalloc-metrics: failed to read page size, defaulting to 4096: {}",
                e
            );
            4096
        },
    };

    thread::Builder::new()
        .name("jemalloc-metrics".into())
        .spawn(move || metrics_loop(page_size))
        .expect("failed to spawn jemalloc-metrics thread");
}

fn read_page_size() -> Result<usize, Error> {
    // Safety: `arenas.page` returns a `size_t`.
    unsafe { jemalloc_ctl::raw::read(b"arenas.page\0") }
}

fn metrics_loop(page_size: usize) {
    loop {
        thread::sleep(COLLECTION_INTERVAL);

        if let Err(e) = collect_once(page_size) {
            aptos_logger::warn!("jemalloc-metrics: collection error: {}", e);
        }
    }
}

fn collect_once(page_size: usize) -> Result<(), Error> {
    // Advance the epoch so subsequent reads return fresh values.
    jemalloc_ctl::epoch::advance()?;

    let gauge = |name: &str, val: usize| {
        JEMALLOC_BYTES.with_label_values(&[name]).set(val as i64);
    };

    gauge("allocated", jemalloc_ctl::stats::allocated::read()?);
    gauge("active", jemalloc_ctl::stats::active::read()?);
    gauge("metadata", jemalloc_ctl::stats::metadata::read()?);
    gauge("resident", jemalloc_ctl::stats::resident::read()?);
    gauge("mapped", jemalloc_ctl::stats::mapped::read()?);
    gauge("retained", jemalloc_ctl::stats::retained::read()?);

    // Safety: `pdirty` and `pmuzzy` return `size_t`.
    let pdirty_key = format!("stats.arenas.{ARENAS_ALL}.pdirty\0");
    let pmuzzy_key = format!("stats.arenas.{ARENAS_ALL}.pmuzzy\0");

    let pdirty: usize = unsafe { jemalloc_ctl::raw::read(pdirty_key.as_bytes())? };
    gauge("dirty", pdirty * page_size);

    let pmuzzy: usize = unsafe { jemalloc_ctl::raw::read(pmuzzy_key.as_bytes())? };
    gauge("muzzy", pmuzzy * page_size);

    Ok(())
}
