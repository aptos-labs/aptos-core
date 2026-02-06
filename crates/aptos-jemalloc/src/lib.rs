// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Sets up jemalloc as the global allocator and configures `malloc_conf`
/// with heap profiling enabled.
///
/// Must be invoked at the top level of a binary crate (`main.rs`).
/// The `malloc_conf` value can be overridden at runtime via the `MALLOC_CONF` env var.
#[macro_export]
macro_rules! setup_jemalloc {
    () => {
        #[cfg(unix)]
        #[global_allocator]
        static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

        /// Can be overridden by setting the `MALLOC_CONF` env var.
        #[allow(unsafe_code, non_upper_case_globals)]
        #[cfg(unix)]
        #[used]
        #[unsafe(no_mangle)]
        pub static mut malloc_conf: *const ::std::ffi::c_char =
            c"abort_conf:true,prof:true,lg_prof_sample:23,percpu_arena:percpu,lg_tcache_max:16,tcache_nslots_large:32,background_thread:true,max_background_threads:4,thp:always,metadata_thp:always,dirty_decay_ms:30000,muzzy_decay_ms:10000"
                .as_ptr()
                .cast();
    };
}
