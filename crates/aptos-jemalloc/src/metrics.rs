// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::warn;
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
                warn!("jemalloc-stats: collection error: {}", e);
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

/// Reads a value of type `T` from a mallctl key under the merged arena namespace.
unsafe fn read_arena<T: Copy>(name: &str) -> Result<T, Error> {
    let key = format!("stats.arenas.{ARENAS_ALL}.{name}\0");
    unsafe { jemalloc_ctl::raw::read(key.as_bytes()) }
}

fn set_gauge(name: &str, val: i64) {
    JEMALLOC_STATS.set_with(&[name], val);
}

fn collect_once() -> Result<(), Error> {
    // Advance the epoch so subsequent reads return fresh values.
    jemalloc_ctl::epoch::advance()?;

    // Global stats (bytes).
    set_gauge(
        "allocated_bytes",
        jemalloc_ctl::stats::allocated::read()? as i64,
    );
    set_gauge("active_bytes", jemalloc_ctl::stats::active::read()? as i64);
    set_gauge(
        "metadata_bytes",
        jemalloc_ctl::stats::metadata::read()? as i64,
    );
    set_gauge(
        "resident_bytes",
        jemalloc_ctl::stats::resident::read()? as i64,
    );
    set_gauge("mapped_bytes", jemalloc_ctl::stats::mapped::read()? as i64);
    set_gauge(
        "retained_bytes",
        jemalloc_ctl::stats::retained::read()? as i64,
    );

    // Per-arena stats aggregated across all arenas (raw mallctl).
    let page_size = page_size();
    unsafe {
        set_gauge(
            "dirty_bytes",
            (read_arena::<usize>("pdirty")? * page_size) as i64,
        );
        set_gauge(
            "muzzy_bytes",
            (read_arena::<usize>("pmuzzy")? * page_size) as i64,
        );
        set_gauge("tcache_bytes", read_arena::<usize>("tcache_bytes")? as i64);
    }

    // Metadata THP: number of transparent huge pages backing jemalloc metadata.
    let n_thp: usize = unsafe { jemalloc_ctl::raw::read(b"stats.metadata_thp\0") }?;
    set_gauge("metadata_thp_pages", n_thp as i64);

    if let Err(e) = collect_hpa_stats(page_size) {
        warn!("jemalloc-stats: HPA collection error: {}", e);
    }

    Ok(())
}

fn collect_hpa_stats(page_size: usize) -> Result<(), Error> {
    unsafe {
        // Hugification / purge counters (monotonic u64).
        set_gauge(
            "hpa_nhugifies",
            read_arena::<u64>("hpa_shard.nhugifies")? as i64,
        );
        set_gauge(
            "hpa_ndehugifies",
            read_arena::<u64>("hpa_shard.ndehugifies")? as i64,
        );
        set_gauge(
            "hpa_npurge_passes",
            read_arena::<u64>("hpa_shard.npurge_passes")? as i64,
        );
        set_gauge(
            "hpa_npurges",
            read_arena::<u64>("hpa_shard.npurges")? as i64,
        );

        // Full slab breakdown (huge vs non-huge).
        set_gauge(
            "hpa_full_slabs_npageslabs_huge",
            read_arena::<usize>("hpa_shard.full_slabs.npageslabs_huge")? as i64,
        );
        set_gauge(
            "hpa_full_slabs_npageslabs_nonhuge",
            read_arena::<usize>("hpa_shard.full_slabs.npageslabs_nonhuge")? as i64,
        );
        set_gauge(
            "hpa_full_slabs_active_huge_bytes",
            (read_arena::<usize>("hpa_shard.full_slabs.nactive_huge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_full_slabs_active_nonhuge_bytes",
            (read_arena::<usize>("hpa_shard.full_slabs.nactive_nonhuge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_full_slabs_dirty_huge_bytes",
            (read_arena::<usize>("hpa_shard.full_slabs.ndirty_huge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_full_slabs_dirty_nonhuge_bytes",
            (read_arena::<usize>("hpa_shard.full_slabs.ndirty_nonhuge")? * page_size) as i64,
        );

        // Empty slab breakdown (huge vs non-huge).
        set_gauge(
            "hpa_empty_slabs_npageslabs_huge",
            read_arena::<usize>("hpa_shard.empty_slabs.npageslabs_huge")? as i64,
        );
        set_gauge(
            "hpa_empty_slabs_npageslabs_nonhuge",
            read_arena::<usize>("hpa_shard.empty_slabs.npageslabs_nonhuge")? as i64,
        );
        set_gauge(
            "hpa_empty_slabs_active_huge_bytes",
            (read_arena::<usize>("hpa_shard.empty_slabs.nactive_huge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_empty_slabs_active_nonhuge_bytes",
            (read_arena::<usize>("hpa_shard.empty_slabs.nactive_nonhuge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_empty_slabs_dirty_huge_bytes",
            (read_arena::<usize>("hpa_shard.empty_slabs.ndirty_huge")? * page_size) as i64,
        );
        set_gauge(
            "hpa_empty_slabs_dirty_nonhuge_bytes",
            (read_arena::<usize>("hpa_shard.empty_slabs.ndirty_nonhuge")? * page_size) as i64,
        );

        // SEC (Small Extent Cache) stats.
        set_gauge(
            "hpa_sec_bytes",
            read_arena::<usize>("hpa_sec_bytes")? as i64,
        );
    }

    Ok(())
}
