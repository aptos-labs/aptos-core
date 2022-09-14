// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Test module.
//!
//! Add new test modules to this list.
//!
//! This is not in a top-level tests directory because each file there gets compiled into a
//! separate binary. The linker ends up repeating a lot of work for each binary to not much
//! benefit.
//!
//! Set env REGENERATE_GOLDENFILES to update the golden files when running tests..

mod account_universe;
mod create_account;
mod data_store;
mod execution_strategies;
mod failed_transaction_tests;
mod genesis;
mod genesis_initializations;
mod mint;
mod module_publishing;
mod on_chain_configs;
mod peer_to_peer;
mod scripts;
mod transaction_fuzzer;
mod verify_txn;
