// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_node::{utils::ERROR_MSG_BAD_FEATURE_FLAGS, AptosNodeArgs};
use clap::Parser;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// Can be overwritten by setting the `MALLOC_CONF` env var.
#[cfg(unix)]
#[unsafe(no_mangle)]
pub static malloc_conf: &[u8] = b"prof:true,lg_prof_sample:23\0";

fn main() {
    // Check that we are not including any Move test natives
    aptos_vm::natives::assert_no_test_natives(ERROR_MSG_BAD_FEATURE_FLAGS);

    // Check that we do have the Move VM's tracing feature enabled
    move_vm_runtime::tracing::assert_move_vm_tracing_feature_disabled(ERROR_MSG_BAD_FEATURE_FLAGS);

    // Start the node
    AptosNodeArgs::parse().run()
}
