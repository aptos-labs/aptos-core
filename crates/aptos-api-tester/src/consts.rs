// Copyright © Aptos Foundation

use once_cell::sync::Lazy;
use std::{env, time::Duration};
use url::Url;

// faucet constants

pub static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());

pub static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());

pub static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.aptoslabs.com").unwrap());

pub static TESTNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.testnet.aptoslabs.com").unwrap());

pub const FUND_AMOUNT: u64 = 100_000_000;

// persistency check constants

pub static PERSISTENCY_TIMEOUT: Lazy<Duration> = Lazy::new(|| {
    env::var("PERSISTENCY_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30))
});

pub static SLEEP_PER_CYCLE: Lazy<Duration> = Lazy::new(|| {
    env::var("SLEEP_PER_CYCLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_millis)
        .unwrap_or(Duration::from_millis(100))
});

// runtime constants

pub static NUM_THREADS: Lazy<usize> = Lazy::new(|| {
    env::var("NUM_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4)
});

pub static STACK_SIZE: Lazy<usize> = Lazy::new(|| {
    env::var("STACK_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4 * 1024 * 1024)
});
