// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
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
mod execution_strategies;
mod genesis;
mod genesis_initializations;
mod invariant_violation;
mod loader;
mod mint;
mod on_chain_configs;
mod peer_to_peer;
mod scripts;
mod state_store;
mod transaction_fuzzer;
mod verify_txn;
