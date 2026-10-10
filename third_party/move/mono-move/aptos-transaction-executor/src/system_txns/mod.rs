// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Execution of system transactions: consensus-produced, unmetered, fee-free,
//! and always expected to succeed — when one fails (unexpectedly), the whole
//! block is aborted. Each kind lives in its own submodule.

mod block_epilogue;
mod block_metadata;
mod common;
