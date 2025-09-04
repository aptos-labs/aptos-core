// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod consts;
mod counters;
mod persistent_check;
mod strings;
mod tests;
mod tokenv1_client;
mod utils;
#[macro_use]
mod macros;

use crate::utils::{NetworkName, TestName};
use anyhow::Result;
use velor_logger::{info, Level, Logger};
use velor_push_metrics::MetricsPusher;
use consts::{NETWORK_NAME, NUM_THREADS, STACK_SIZE};
use futures::future::join_all;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::{Builder, Runtime};

async fn test_flows(runtime: &Runtime, network_name: NetworkName) -> Result<()> {
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs()
        .to_string();
    info!(
        "----- STARTING TESTS FOR {} WITH RUN ID {} -----",
        network_name.to_string(),
        run_id
    );

    // Flow 1: New account
    let test_time = run_id.clone();
    let handle_newaccount = runtime.spawn(async move {
        TestName::NewAccount.run(network_name, &test_time).await;
    });

    // Flow 2: Coin transfer
    let test_time = run_id.clone();
    let handle_cointransfer = runtime.spawn(async move {
        TestName::CoinTransfer.run(network_name, &test_time).await;
    });

    // Flow 3: NFT transfer
    let test_time = run_id.clone();
    let handle_nfttransfer = runtime.spawn(async move {
        TestName::TokenV1Transfer
            .run(network_name, &test_time)
            .await;
    });

    // Flow 4: Publishing module
    let test_time = run_id.clone();
    let handle_publishmodule = runtime.spawn(async move {
        TestName::PublishModule.run(network_name, &test_time).await;
    });

    // Flow 5: View function
    let test_time = run_id.clone();
    let handle_viewfunction = runtime.spawn(async move {
        TestName::ViewFunction.run(network_name, &test_time).await;
    });

    join_all(vec![
        handle_newaccount,
        handle_cointransfer,
        handle_nfttransfer,
        handle_publishmodule,
        handle_viewfunction,
    ])
    .await;
    Ok(())
}

fn main() -> Result<()> {
    // create runtime
    let runtime = Builder::new_multi_thread()
        .worker_threads(*NUM_THREADS)
        .enable_all()
        .thread_stack_size(*STACK_SIZE)
        .build()?;

    // log metrics
    Logger::builder().level(Level::Info).build();
    let _mp = MetricsPusher::start_for_local_run("api-tester");

    // run tests
    runtime.block_on(async {
        let _ = test_flows(&runtime, *NETWORK_NAME).await;
    });

    Ok(())
}
