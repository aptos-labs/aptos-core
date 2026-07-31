// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod resident;
#[cfg(unix)]
pub use jemallocator;
pub use resident::current_process_resident_bytes;

#[cfg(unix)]
mod metrics;
#[cfg(unix)]
pub use metrics::start_jemalloc_metrics_thread;

/// Current live heap bytes attributable to the calling thread, as
/// `thread.allocated − thread.deallocated` from jemalloc's per-thread counters.
///
/// Signed because a thread may deallocate memory allocated on another thread, so
/// its `deallocated` can momentarily exceed its `allocated`. Callers snapshot
/// this at the start of a synchronous, `.await`-free region and re-read it to get
/// the region's allocation delta. The mallctl MIBs are cached per-thread so each
/// call is a couple of instructions.
///
/// Returns `0` when the `metered-allocations` feature is off (or on non-unix),
/// which makes any `Meter` built on it a no-op.
#[cfg(all(unix, feature = "metered-allocations"))]
pub fn current_live_bytes() -> i64 {
    use jemalloc_ctl::thread;
    thread_local! {
        static MIBS: (thread::allocatedp_mib, thread::deallocatedp_mib) = (
            thread::allocatedp::mib().expect("jemalloc thread.allocated mib"),
            thread::deallocatedp::mib().expect("jemalloc thread.deallocated mib"),
        );
    }
    MIBS.with(|(alloc, dealloc)| match (alloc.read(), dealloc.read()) {
        (Ok(a), Ok(d)) => a.get() as i64 - d.get() as i64,
        _ => 0,
    })
}

/// No-op reader: see the metered variant above.
#[cfg(not(all(unix, feature = "metered-allocations")))]
pub fn current_live_bytes() -> i64 {
    0
}

/// Sets up jemalloc as the global allocator and configures `malloc_conf`.
///
/// Invoke at the top level of a binary crate (`main.rs`).
/// The configuration can be overridden at runtime via the `MALLOC_CONF` environment variable.
///
/// Configuration notes:
///   - `prof:true,lg_prof_sample:23` -- Heap profiling with an 8 MiB sampling interval.
///   - `percpu_arena:percpu` -- Per-CPU arenas to reduce cross-thread contention.
///     Also the default arena count without this setting (4x #CPUs) can be a bit excessive.
///   - `hpa:true,metadata_thp:auto` -- Use Huge Pages to reduce dTLB misses (ignored on macOS).
///   - `background_thread:true,max_background_threads:4` -- Background purging threads.
///   - `dirty_decay_ms:30000,muzzy_decay_ms:120000` -- Longer decay trades RSS for
///     fewer `madvise` syscalls.
///   - `lg_tcache_max:16,tcache_nslots_large:32` -- 64 KiB thread-cache ceiling,
///     since 40 KiB allocations are very hot (as of 02/06/2026) in the node.
#[macro_export]
macro_rules! setup_jemalloc {
    () => {
        #[cfg(unix)]
        #[global_allocator]
        static ALLOC: $crate::jemallocator::Jemalloc = $crate::jemallocator::Jemalloc;

        #[allow(unsafe_code, non_upper_case_globals)]
        #[cfg(unix)]
        #[used]
        #[unsafe(no_mangle)]
        pub static mut malloc_conf: *const ::std::ffi::c_char = c"\
              prof:true,lg_prof_sample:23,\
              percpu_arena:percpu,\
              hpa:true,metadata_thp:auto,\
              background_thread:true,max_background_threads:4,\
              dirty_decay_ms:30000,muzzy_decay_ms:120000,\
              lg_tcache_max:16,tcache_nslots_large:32"
            .as_ptr()
            .cast();
    };
}
