// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Shared utilities for Aptos Move testing tools.
//!
//! This library provides common functionality for tools like aptos-e2e-comparison-testing
//! and replay-benchmark, including:
//! - Transaction output comparison and diff generation
//! - Aptos framework package management
//! - Compilation caching
//! - REST client initialization
//! - Data persistence utilities

pub mod client;
pub mod compilation;
pub mod diff;
pub mod execution;
pub mod packages;
pub mod persistence;
pub mod state_store_utils;

// Re-export commonly used types
pub use client::{create_client_and_debugger, create_debugger, create_rest_client, ClientConfig};
pub use compilation::{CompilationCache, PackageInfo};
pub use diff::{Diff, TransactionDiff, TransactionDiffBuilder};
pub use execution::VMExecutor;
pub use packages::{
    check_aptos_packages_availability, download_aptos_packages, get_aptos_dir, is_aptos_package,
    prepare_aptos_packages, APTOS_PACKAGES_DIR_NAMES,
};
pub use persistence::{
    deserialize_blocks, load_blocks_from_file, save_blocks_to_file, serialize_blocks,
    TransactionBlock,
};
pub use state_store_utils::{
    populate_state_with_aptos_packages, populate_state_with_packages, StateStorePackageInfo,
};
