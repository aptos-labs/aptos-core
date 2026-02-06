// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Sets up jemalloc as the global allocator and configures `malloc_conf`.
///
/// Must be invoked at the top level of a binary crate (`main.rs`).
/// The `malloc_conf` value can be overridden at runtime via the `MALLOC_CONF` env var.
#[macro_export]
macro_rules! setup_jemalloc {
    () => {
        #[cfg(unix)]
        #[global_allocator]
        static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

        #[allow(unsafe_code, non_upper_case_globals)]
        #[cfg(unix)]
        #[used]
        #[unsafe(no_mangle)]
        pub static mut malloc_conf: *const ::std::ffi::c_char = c"abort_conf:true,\
              percpu_arena:phycpu,\
              prof:true,\
              lg_prof_sample:23"
            .as_ptr()
            .cast();
    };
}
