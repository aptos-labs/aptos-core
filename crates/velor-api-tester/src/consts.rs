// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::NetworkName;
use once_cell::sync::Lazy;
use std::{env, time::Duration};
use url::Url;

// Node and faucet constants

// TODO: consider making this a CLI argument
pub static NETWORK_NAME: Lazy<NetworkName> = Lazy::new(|| {
    env::var("NETWORK_NAME")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(NetworkName::Devnet)
});

pub static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.velorlabs.com").unwrap());

pub static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.velorlabs.com").unwrap());

pub static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.velorlabs.com").unwrap());

pub static TESTNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.testnet.velorlabs.com").unwrap());

pub const FUND_AMOUNT: u64 = 100_000_000;

// Persistency check constants

// How long a persistent check runs for.
pub static PERSISTENCY_TIMEOUT: Lazy<Duration> = Lazy::new(|| {
    env::var("PERSISTENCY_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30))
});

// Wait time between tries during a persistent check.
pub static SLEEP_PER_CYCLE: Lazy<Duration> = Lazy::new(|| {
    env::var("SLEEP_PER_CYCLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(100))
});

// Runtime constants

// The number of threads to use for running tests.
pub static NUM_THREADS: Lazy<usize> = Lazy::new(|| {
    env::var("NUM_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
});

// The size of the stack for each thread.
pub static STACK_SIZE: Lazy<usize> = Lazy::new(|| {
    env::var("STACK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4 * 1024 * 1024)
});
