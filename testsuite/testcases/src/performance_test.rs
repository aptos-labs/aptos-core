// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use anyhow::anyhow;
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, NodeExt, Result, Test};
use aptos_logger::{debug, error, info, sample, sample::SampleRate};
use aptos_rest_client::aptos_api_types::AccountSignature::Ed25519Signature;
use aptos_sdk::{
    crypto::{
        ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
        SigningKey, Uniform,
    },
    transaction_builder::aptos_stdlib::aptos_coin_transfer,
};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        authenticator::AccountAuthenticator, EntryFunction, RawTransaction, Script,
        SignedTransaction, TransactionPayload,
    },
    PeerId,
};
use async_trait::async_trait;
use balter::{prelude::ConfigurableScenario, scenario, transaction};
use futures::{stream::FuturesUnordered, StreamExt};
use rand::{thread_rng, RngCore};
use reqwest::{StatusCode, Url};
use std::{
    cell::OnceCell,
    ops::Add,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, OnceLock,
    },
    time::{Duration, Instant},
};

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkLoadTest for PerformanceBenchmark {}

#[async_trait]
impl NetworkTest for PerformanceBenchmark {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

pub struct ConsensusOnlyBenchmark {
    pub test_time: Duration,
    pub concurrency: usize,
}

impl Test for ConsensusOnlyBenchmark {
    fn name(&self) -> &'static str {
        "consensus-only benchmark"
    }
}

const MAX_BATCH_SIZE: usize = 1;

#[async_trait]
impl NetworkTest for ConsensusOnlyBenchmark {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let ctx = ctx.ctx.lock().await;

        // Get all URLs
        let clients: Vec<_> = ctx
            .swarm
            .read()
            .await
            .validators()
            .map(|val| val.rest_client())
            .collect();

        // Create Balter
        // BALTER_CONTEXT
        //     .set(BalterContext {
        //         clients,
        //         idx: AtomicU64::new(0),
        //         batch_size: MAX_BATCH_SIZE,
        //     })
        //     .map_err(|_| anyhow!("couldn't set context"))
        //     .unwrap();

        // let result = batch_load_test()
        //     .tps(10000)
        //     .duration(Duration::from_secs(600))
        //     .error_rate(0.0)
        //     .hint(balter::Hint::Concurrency(20000))
        //     .await;

        // let concurrency = self.concurrency;
        // let test_time = self.test_time;
        // let mut futures = Vec::new();
        // for i in 0..concurrency {
        //     if i % 100 == 0 {
        //         tokio::time::sleep(Duration::from_millis(300)).await;
        //     }
        //     if i == 10000 {
        //         tokio::time::sleep(Duration::from_secs(30)).await;
        //     }
        //     futures.push(tokio::spawn(async move {
        //         tokio::time::timeout(test_time, batch_load_test()).await
        //     }));
        // }
        // let _result = futures::future::join_all(futures).await;

        let mut futures = Vec::new();
        let tps_per_client = (self.concurrency / clients.len()).max(1);
        let test_time = self.test_time;
        let global_wait_until = Instant::now().add(Duration::from_secs(30));
        println!(
            "num_clients {}, tps_per_client {}, test_time {:?}, global_wait_till {:?}",
            clients.len(),
            tps_per_client,
            test_time,
            global_wait_until
        );
        for (id, client) in clients.into_iter().enumerate() {
            let id = id as u8;
            let req_client = UniformPerValidatorRateClient::new(id, client, tps_per_client as u64);
            futures.push(tokio::spawn(async move {
                tokio::time::timeout(test_time, req_client.run(global_wait_until)).await
            }));
        }
        let _result = futures::future::join_all(futures).await;

        info!("test complete");

        // let result = tokio::time::timeout(Duration::from_secs(60), load_test()).await;

        // info!("{:?}", result);
        // println!("{:?}", result);

        Ok(())
    }
}

static BALTER_CONTEXT: OnceLock<BalterContext> = OnceLock::new();

pub struct BalterContext {
    clients: Vec<aptos_rest_client::Client>,
    idx: AtomicU64,
    batch_size: usize,
}

impl BalterContext {
    fn next_client(&self) -> aptos_rest_client::Client {
        let idx = self.idx.fetch_add(1, Ordering::Relaxed) % self.clients.len() as u64;
        self.clients[idx as usize].clone()
    }
}

#[scenario]
async fn load_test() {
    let client = { BALTER_CONTEXT.get().unwrap().next_client() };
    let (txn_tx, mut txn_rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(async move {
        let mut seq_num = 0;
        let sender = PeerId::random();
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key: Ed25519PublicKey = (&private_key).into();
        let sig = private_key.sign_arbitrary_message(&[]);
        loop {
            let txn = SignedTransaction::new_single_sender(
                RawTransaction::new(
                    sender,
                    seq_num,
                    aptos_coin_transfer(sender, 100),
                    0,
                    0,
                    Duration::from_secs(60).as_secs(),
                    ChainId::test(),
                ),
                AccountAuthenticator::ed25519(public_key.clone(), sig.clone()),
            );
            txn_tx.send(txn).await.ok();
            seq_num = seq_num + 1;
        }
    });
    while let Some(txn) = txn_rx.recv().await {
        let txn_payload = bcs::to_bytes(&txn).unwrap();
        let _ = transaction(&client, txn_payload).await;
    }
}

#[transaction]
async fn transaction(
    client: &aptos_rest_client::Client,
    txn_payload: Vec<u8>,
) -> anyhow::Result<()> {
    let res = client
        .post(client.build_path("submit_txn").unwrap())
        .body(txn_payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await?;

    if res.status() != StatusCode::NOT_FOUND {
        let _ = res.error_for_status()?;
    }

    Ok(())
}

// #[scenario]
async fn batch_load_test() {
    let (client, batch_size) = {
        let ctx = BALTER_CONTEXT.get().unwrap();
        (ctx.next_client(), ctx.batch_size)
    };
    let (txn_tx, mut txn_rx) = tokio::sync::mpsc::channel(100);
    tokio::spawn(async move {
        let mut seq_num = 0;
        let sender = PeerId::random();
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key: Ed25519PublicKey = (&private_key).into();
        let sig = private_key.sign_arbitrary_message(&[]);
        loop {
            let mut batch = Vec::new();
            for i in 0..batch_size {
                let txn = SignedTransaction::new_single_sender(
                    RawTransaction::new(
                        sender,
                        seq_num,
                        aptos_coin_transfer(sender, 100),
                        0,
                        0,
                        Duration::from_secs(60).as_secs(),
                        ChainId::test(),
                    ),
                    AccountAuthenticator::ed25519(public_key.clone(), sig.clone()),
                );
                batch.push(txn);
                seq_num = seq_num + 1;
            }
            txn_tx.send(batch).await.ok();
        }
    });
    while let Some(batch_txn) = txn_rx.recv().await {
        let txn_payload = bcs::to_bytes(&batch_txn).unwrap();
        let _ = batch_transaction(&client, txn_payload).await;
    }
}

// #[transaction]
async fn batch_transaction(
    client: &aptos_rest_client::Client,
    txn_payload: Vec<u8>,
) -> anyhow::Result<()> {
    let res = client
        .post(client.build_path("submit_txn_batch").unwrap())
        .body(txn_payload)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;

    if res.status() != StatusCode::NOT_FOUND {
        let _ = res.error_for_status()?;
    }

    Ok(())
}

pub struct UniformPerValidatorRateClient {
    client_id: u8,
    client: aptos_rest_client::Client,
    requests_per_second: u64,
}

impl UniformPerValidatorRateClient {
    pub fn new(client_id: u8, client: aptos_rest_client::Client, requests_per_second: u64) -> Self {
        Self {
            client_id,
            client,
            requests_per_second,
        }
    }

    pub async fn run(self, wait_until: Instant) -> Result<()> {
        // Calculate the delay between requests in milliseconds
        let delay_us = 1_000_000 / self.requests_per_second;
        let total_slots = self.requests_per_second;
        let mut handles = Vec::new();
        let slow_requests = Arc::new(AtomicU64::new(0));

        println!("Uniform performance test: Client {}", self.client_id);
        println!("delay_us {}, total_slots {}", delay_us, total_slots);

        let slow_reqs_clone = slow_requests.clone();
        let id = self.client_id;
        tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(10));
            loop {
                _ = timer.tick().await;
                println!(
                    "Uniform performance test: Client {}, Slow Reqs: {}",
                    id,
                    slow_reqs_clone.load(Ordering::Relaxed)
                );
            }
        });

        // Client latency multiplier
        let wait_until = wait_until + Duration::from_secs(1) * self.client_id as u32;

        // Create tasks for each "slot" in a second
        for slot in 0..total_slots {
            let client = self.client.clone();
            let client_id = self.client_id;
            let wait_until_multiplier = slot % 20;
            let wait_until = wait_until + (Duration::from_secs(10) * wait_until_multiplier as u32);
            let slow_requests = slow_requests.clone();

            let handle = tokio::spawn(async move {
                let (txn_tx, mut txn_rx) = tokio::sync::mpsc::channel(100);
                tokio::spawn(async move {
                    let mut seq_num = 0;
                    let sender = PeerId::random();
                    let private_key = Ed25519PrivateKey::generate_for_testing();
                    let public_key: Ed25519PublicKey = (&private_key).into();
                    let sig = private_key.sign_arbitrary_message(&[]);
                    loop {
                        let mut batch = Vec::with_capacity(MAX_BATCH_SIZE);
                        for i in 0..MAX_BATCH_SIZE {
                            let txn = SignedTransaction::new_single_sender(
                                RawTransaction::new(
                                    sender,
                                    seq_num,
                                    aptos_coin_transfer(sender, 100),
                                    0,
                                    0,
                                    Duration::from_secs(60).as_secs(),
                                    ChainId::test(),
                                ),
                                // AccountAuthenticator::ed25519(public_key.clone(), sig.clone()),
                                AccountAuthenticator::NoAccountAuthenticator,
                            );
                            batch.push(txn);
                            seq_num = seq_num + 1;
                        }
                        let txn_payload = bcs::to_bytes(&batch).unwrap();
                        txn_tx.send(txn_payload).await.ok();
                    }
                });

                let initial_delay = Duration::from_micros(slot * delay_us);
                let wait_until = wait_until.add(initial_delay).into();
                tokio::time::sleep_until(wait_until).await;

                let mut interval = tokio::time::interval(Duration::from_secs(1));
                let mut reporting_interval = tokio::time::interval(Duration::from_secs(60));
                let mut futures = FuturesUnordered::new();
                let mut failed_count = 0;
                let mut success_count = 0;
                loop {
                    let start = Instant::now();

                    tokio::select! {
                        biased;
                         _ = interval.tick() => {
                            let Some(txn_payload) = txn_rx.recv().await else {
                                return;
                            };
                            let fut = batch_transaction(&client, txn_payload);
                            futures.push(fut);
                        },
                        Some(result) = futures.next() => {
                            match result {
                                Ok(_) => {
                                    success_count += 1;
                                },
                                Err(e) => {
                                failed_count += 1;
                                    sample!(
                                        SampleRate::Duration(Duration::from_secs(1)),
                                        error!("Slot {}: Request failed {}", slot, e)
                                    );
                                },
                            }
                        },
                        _ = reporting_interval.tick() => {
                            // println!("Client {} Slot {}: Success {}, Failed: {}", client_id, slot, success_count, failed_count);
                        }
                    }

                    // Log if we're falling behind
                    let elapsed = start.elapsed();
                    if elapsed > Duration::from_millis(1005) {
                        slow_requests.fetch_add(1, Ordering::Relaxed);
                        sample!(
                            SampleRate::Duration(Duration::from_secs(10)),
                            println!(
                                "Warning: Client {} Slot {} is falling behind, took {}ms",
                                client_id,
                                slot,
                                elapsed.as_millis()
                            )
                        );
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all tasks to complete (they won't unless there's an error)
        for handle in handles {
            handle.await?;
        }

        Ok(())
    }
}

#[test]
fn test_txn_size() {
    let sender = PeerId::random();
    let seq_num = 1;
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key: Ed25519PublicKey = (&private_key).into();
    let sig = private_key.sign_arbitrary_message(&[]);
    let txn = SignedTransaction::new_single_sender(
        RawTransaction::new(
            sender,
            seq_num,
            // aptos_coin_transfer(sender, 100),
            TransactionPayload::Script(Script::new(vec![], vec![], vec![])),
            0,
            0,
            Duration::from_secs(60).as_secs(),
            ChainId::test(),
        ),
        // AccountAuthenticator::ed25519(public_key.clone(), sig.clone()),
        AccountAuthenticator::NoAccountAuthenticator,
    );
    let txn_bytes = bcs::to_bytes(&txn).unwrap();
    println!("{:?}", txn_bytes.len());
}
