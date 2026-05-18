// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{ed25519::Ed25519PrivateKey, Uniform};
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, NodeExt, Result, Test};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use async_trait::async_trait;
use log::info;
use rand::SeedableRng;
use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

/// Fixed seed so the same accounts are generated in both the config override
/// (before nodes start) and the test body (after nodes start).
const TRACING_ACCOUNT_SEED: u64 = 0xDEAD_BEEF_CAFE_1234;
/// Need enough accounts to sustain 500 TPS without sequence number bottleneck.
/// Each account sends ~10 TPS → 50 accounts for 500 TPS.
const NUM_TRACED_ACCOUNTS: usize = 50;
const TRACED_TPS: u64 = 500;

/// Generate deterministic accounts from a fixed seed.
/// Used in both config override and test body to get the same addresses.
pub fn generate_traced_accounts(num: usize) -> Vec<LocalAccount> {
    let mut rng: rand::rngs::StdRng = SeedableRng::seed_from_u64(TRACING_ACCOUNT_SEED);
    (0..num)
        .map(|_| {
            let key = Ed25519PrivateKey::generate(&mut rng);
            let pubkey = aptos_crypto::ed25519::Ed25519PublicKey::from(&key);
            let address = AuthenticationKey::ed25519(&pubkey).account_address();
            LocalAccount::new(address, AccountKey::from_private_key(key), 0)
        })
        .collect()
}

/// Return just the addresses (for use in config override).
pub fn traced_account_addresses() -> Vec<AccountAddress> {
    generate_traced_accounts(NUM_TRACED_ACCOUNTS)
        .iter()
        .map(|a| a.address())
        .collect()
}

/// Wraps an inner NetworkTest and adds 500 TPS traced traffic alongside it.
/// The inner test runs the main load (e.g. TwoTrafficsTest with MaxLoad);
/// traced traffic runs concurrently from pre-generated accounts.
pub struct TransactionTracingTest {
    pub inner: Box<dyn NetworkTest>,
}

impl Test for TransactionTracingTest {
    fn name(&self) -> &'static str {
        "transaction_tracing_test"
    }
}

/// Spawn a background task that submits traced transactions at a constant TPS
/// from pre-funded accounts, distributed across multiple validator REST clients.
fn spawn_traced_traffic(
    accounts: Vec<LocalAccount>,
    receiver: AccountAddress,
    rest_clients: Vec<RestClient>,
    txn_factory: aptos_sdk::transaction_builder::TransactionFactory,
    tps: u64,
    stop: Arc<AtomicBool>,
    submitted: Arc<AtomicU64>,
    failed: Arc<AtomicU64>,
) -> tokio::task::JoinHandle<Vec<LocalAccount>> {
    tokio::spawn(async move {
        let mut accounts = accounts;
        let num_accounts = accounts.len();
        let num_clients = rest_clients.len();
        let interval = Duration::from_micros(1_000_000 / tps);
        let mut idx = 0usize;

        while !stop.load(Ordering::Relaxed) {
            let account = &mut accounts[idx % num_accounts];
            let client = &rest_clients[idx % num_clients];

            let tx = account.sign_with_transaction_builder(
                txn_factory
                    .clone()
                    .payload(aptos_stdlib::aptos_coin_transfer(receiver, 100)),
            );

            match client.submit(&tx).await {
                Ok(_) => {
                    submitted.fetch_add(1, Ordering::Relaxed);
                },
                Err(_) => {
                    failed.fetch_add(1, Ordering::Relaxed);
                },
            }

            idx += 1;
            tokio::time::sleep(interval).await;
        }

        accounts
    })
}

#[async_trait]
impl NetworkTest for TransactionTracingTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        // Step 1: Fund traced accounts and start 500 TPS traced traffic.
        let (stop, traced_submitted, traced_failed, handle, traffic_start) = {
            let mut ctx_locker = ctx.ctx.lock().await;
            let ctx = ctx_locker.deref_mut();

            let traced_accounts = generate_traced_accounts(NUM_TRACED_ACCOUNTS);
            info!(
                "TxnTracing: generated {} deterministic traced accounts",
                traced_accounts.len()
            );

            let mut pub_info = ctx.swarm.read().await.aptos_public_info();
            for account in &traced_accounts {
                pub_info.mint(account.address(), 100_000_000_000).await?;
            }
            let receiver = pub_info
                .create_and_fund_user_account(10_000_000_000)
                .await?;
            info!(
                "TxnTracing: funded {} traced accounts, receiver={}",
                traced_accounts.len(),
                receiver.address()
            );

            // Submit traced traffic to only 1 validator so we can compare its
            // mempool/QS latency against other validators that receive only
            // untraced traffic. Validator 0 = traced, validators 1..N = control.
            let rest_clients: Vec<RestClient> = {
                let swarm = ctx.swarm.read().await;
                swarm
                    .validators()
                    .take(1)
                    .map(|v| v.rest_client())
                    .collect()
            };
            let txn_factory = pub_info.transaction_factory();

            info!(
                "TxnTracing: starting traced traffic at {} TPS ({} accounts, {} validators)",
                TRACED_TPS,
                NUM_TRACED_ACCOUNTS,
                rest_clients.len(),
            );

            let stop = Arc::new(AtomicBool::new(false));
            let traced_submitted = Arc::new(AtomicU64::new(0));
            let traced_failed = Arc::new(AtomicU64::new(0));

            let handle = spawn_traced_traffic(
                traced_accounts,
                receiver.address(),
                rest_clients,
                txn_factory,
                TRACED_TPS,
                stop.clone(),
                traced_submitted.clone(),
                traced_failed.clone(),
            );

            (
                stop,
                traced_submitted,
                traced_failed,
                handle,
                std::time::Instant::now(),
            )
        };

        // Step 2: Run the inner test (TwoTrafficsTest with MaxLoad + geo-distribution).
        // This handles the main high-throughput traffic and success criteria.
        info!("TxnTracing: starting inner network test (land-blocking load)");
        let inner_result = self.inner.run(ctx).await;

        // Step 3: Stop traced traffic and report stats.
        stop.store(true, Ordering::Relaxed);
        let _accounts = handle.await?;

        let elapsed = traffic_start.elapsed();
        let submitted = traced_submitted.load(Ordering::Relaxed);
        let failed = traced_failed.load(Ordering::Relaxed);
        info!(
            "TxnTracing: traced traffic: submitted={} failed={} effective_tps={:.0} duration={}s",
            submitted,
            failed,
            submitted as f64 / elapsed.as_secs_f64(),
            elapsed.as_secs()
        );
        info!("===== TRANSACTION TRACING TEST RESULTS =====");
        info!("TxnTracing: search Humio for 'TxnTrace' to find traced transaction logs");
        info!("=============================================");

        // Propagate the inner test result
        inner_result
    }
}
