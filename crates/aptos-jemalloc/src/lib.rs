// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(unix)]
pub use jemallocator;

#[cfg(unix)]
mod metrics;
#[cfg(unix)]
pub use metrics::start_jemalloc_metrics_thread;

/// Shared body of the `setup_jemalloc*` macros. `$hpa` is the only setting that
/// differs between profiles; everything else is deliberately identical so the two
/// cannot drift apart.
///
/// Configuration notes:
///   - `prof:true,lg_prof_sample:23` -- Heap profiling with an 8 MiB sampling interval.
///   - `percpu_arena:percpu` -- Per-CPU arenas to reduce cross-thread contention.
///     Also the default arena count without this setting (4x #CPUs) can be a bit excessive.
///   - `hpa,metadata_thp:auto` -- Use Huge Pages to reduce dTLB misses (ignored on macOS).
///   - `background_thread:true,max_background_threads:4` -- Background purging threads.
///   - `dirty_decay_ms:30000,muzzy_decay_ms:120000` -- Longer decay trades RSS for
///     fewer `madvise` syscalls.
///   - `lg_tcache_max:16,tcache_nslots_large:32` -- 64 KiB thread-cache ceiling,
///     since 40 KiB allocations are very hot (as of 02/06/2026) in the node.
#[macro_export]
#[doc(hidden)]
macro_rules! __setup_jemalloc_with_hpa {
    ($hpa:literal) => {
        #[cfg(unix)]
        #[global_allocator]
        static ALLOC: $crate::jemallocator::Jemalloc = $crate::jemallocator::Jemalloc;

        // `concat!` yields a plain `str`, so the NUL terminator is added explicitly.
        #[allow(unsafe_code, non_upper_case_globals)]
        #[cfg(unix)]
        #[used]
        #[unsafe(no_mangle)]
        pub static mut malloc_conf: *const ::std::ffi::c_char = concat!(
            "prof:true,lg_prof_sample:23,",
            "percpu_arena:percpu,",
            $hpa,
            ",metadata_thp:auto,",
            "background_thread:true,max_background_threads:4,",
            "dirty_decay_ms:30000,muzzy_decay_ms:120000,",
            "lg_tcache_max:16,tcache_nslots_large:32\0"
        )
        .as_ptr()
        .cast();
    };
}

/// Sets up jemalloc as the global allocator and configures `malloc_conf` for the node.
///
/// Invoke at the top level of a binary crate (`main.rs`).
/// The configuration can be overridden at runtime via the `MALLOC_CONF` environment variable.
///
/// Enables the huge page allocator. Use [`setup_jemalloc_for_tools`] instead for anything
/// that is not the node or a benchmark standing in for it.
#[macro_export]
macro_rules! setup_jemalloc {
    () => {
        $crate::__setup_jemalloc_with_hpa!("hpa:true");
    };
}

/// Same as [`setup_jemalloc`], but with jemalloc's huge page allocator disabled.
///
/// HPA has livelocked in `hpa_shard_maybe_do_deferred_work` in tool workloads,
/// pinning a core indefinitely. Use [`setup_jemalloc`] for the node and node-like
/// benchmarks.
///
/// This may already be fixed in the latest jemalloc; retest HPA when upgrading.
#[macro_export]
macro_rules! setup_jemalloc_for_tools {
    () => {
        $crate::__setup_jemalloc_with_hpa!("hpa:false");
    };
}
