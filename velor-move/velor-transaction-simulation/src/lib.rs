// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This is a crate providing unified infrastructure for transaction simulation.
//!
//! ## Overview
//! As of today, this crate defines [`SimulationStateStore`], a standardized trait
//! for state store operations, along with modular implementations for different
//! simulation needs.
//!
//! In the future, we plan to extend this crate with additional abstractions and
//! implementations around the executor.
//!
//! ## Available Implementations
//! - State Views (read-only)
//!   - [`EmptyStateView`]
//!   - [`EitherStateView`]
//! - State Stores (read & write)
//!   - [`InMemoryStateStore`]
//!   - [`DeltaStateStore`]
//!
//! ## Usage
//! To perform transaction-based simulations, it is recommended to use [`SimulationStateStore`] to:
//! - Ensure portability across different implementations.
//! - Leverage built-in utility functions for streamlined resource/config access.

mod account;
mod genesis;
mod state_store;

pub use account::{
    Account, AccountData, AccountPublicKey, CoinStore, FungibleStore, TransactionBuilder,
};
pub use genesis::{
    GENESIS_CHANGE_SET_HEAD, GENESIS_CHANGE_SET_MAINNET, GENESIS_CHANGE_SET_TESTNET,
};
pub use state_store::{
    DeltaStateStore, EitherStateView, EmptyStateView, InMemoryStateStore, SimulationStateStore,
};
