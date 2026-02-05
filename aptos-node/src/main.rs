// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![deny(unsafe_code)]

use aptos_node::{utils::ERROR_MSG_BAD_FEATURE_FLAGS, AptosNodeArgs};
use clap::Parser;
use std::ffi::c_char;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// Can be overridden by setting the `MALLOC_CONF` env var.
#[allow(unsafe_code)]
#[cfg(unix)]
#[used]
#[unsafe(no_mangle)]
pub static mut malloc_conf: *const c_char = c"abort_conf:true,prof:true,lg_prof_sample:23,percpu_arena:percpu,lg_tcache_max:16,tcache_nslots_large:32,background_thread:true,max_background_threads:4,metadata_thp:auto,dirty_decay_ms:30000,muzzy_decay_ms:10000".as_ptr().cast();

fn main() {
    // Check that we are not including any Move test natives
    aptos_vm::natives::assert_no_test_natives(ERROR_MSG_BAD_FEATURE_FLAGS);

    // Start the node
    AptosNodeArgs::parse().run()
}
