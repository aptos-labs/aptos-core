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
pub static mut malloc_conf: *const c_char = c"prof:true,lg_prof_sample:23".as_ptr().cast();

fn main() {
    // Check that we are not including any Move test natives
    aptos_vm::natives::assert_no_test_natives(ERROR_MSG_BAD_FEATURE_FLAGS);

    // Start the node
    AptosNodeArgs::parse().run()
}
