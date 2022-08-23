// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod account_minter;
pub mod stats;
pub mod submission_worker;

use ::aptos_logger::*;
use again::RetryPolicy;
use anyhow::{anyhow, format_err, Result};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use futures::future::{try_join_all, FutureExt};
use itertools::zip;
use once_cell::sync::Lazy;
use rand::prelude::SliceRandom;
use rand_core::SeedableRng;
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{runtime::Handle, task::JoinHandle, time};

use crate::{
    args::TransactionType,
    emitter::{account_minter::AccountMinter, submission_worker::SubmissionWorker},
    transaction_generator::{
        account_generator::AccountGeneratorCreator, nft_mint::NFTMintGeneratorCreator,
        p2p_transaction_generator::P2PTransactionGeneratorCreator, TransactionGeneratorCreator,
    },
};
use aptos_sdk::transaction_builder::aptos_stdlib;
use rand::rngs::StdRng;
use stats::{StatsAccumulator, TxnStats};

// Max is 100k TPS for a full day.
const MAX_TXNS: u64 = 100_000_000_000;
const SEND_AMOUNT: u64 = 1;

// This retry policy is used for important client calls necessary for setting
// up the test (e.g. account creation) and collecting its results (e.g. checking
// account sequence numbers). If these fail, the whole test fails. We do not use
// this for submitting transactions, as we have a way to handle when that fails.
// This retry policy means an operation will take 8 seconds at most.
static RETRY_POLICY: Lazy<RetryPolicy> = Lazy::new(|| {
    RetryPolicy::exponential(Duration::from_millis(125))
        .with_max_retries(6)
        .with_jitter(true)
});

#[derive(Clone, Debug)]
pub struct EmitModeParams {
    pub txn_expiration_time_secs: u64,
    pub check_stats_at_end: bool,

    pub workers_per_endpoint: usize,
    pub accounts_per_worker: usize,

    /// Max transactions per account in mempool
    pub transactions_per_account: usize,
    pub max_submit_batch_size: usize,
    pub start_offset_multiplier_millis: f64,
    pub start_jitter_millis: u64,
    pub wait_millis: u64,
    pub wait_committed: bool,
    pub check_account_sequence_only_once: bool,
}

#[derive(Clone, Debug)]
pub struct EmitJobRequest {
    rest_clients: Vec<RestClient>,
    mode: EmitJobMode,

    gas_price: u64,
    invalid_transaction_ratio: usize,
    pub duration: Duration,
    reuse_accounts: bool,
    transaction_type: TransactionType,

    txn_expiration_time_secs: u64,
    check_stats_at_end: bool,
}

#[derive(Clone, Debug)]
pub enum EmitJobMode {
    MaxLoad { mempool_backlog: usize },
    ConstTps { tps: usize },
}

impl Default for EmitJobRequest {
    fn default() -> Self {
        Self {
            rest_clients: Vec::new(),
            mode: EmitJobMode::MaxLoad {
                mempool_backlog: 3000,
            },
            gas_price: 0,
            invalid_transaction_ratio: 0,
            duration: Duration::from_secs(300),
            reuse_accounts: false,
            transaction_type: TransactionType::P2P,
            txn_expiration_time_secs: 60,
            check_stats_at_end: true,
        }
    }
}

impl EmitJobRequest {
    pub fn new(rest_clients: Vec<RestClient>) -> Self {
        Self::default().rest_clients(rest_clients)
    }

    pub fn rest_clients(mut self, rest_clients: Vec<RestClient>) -> Self {
        self.rest_clients = rest_clients;
        self
    }

    pub fn gas_price(mut self, gas_price: u64) -> Self {
        self.gas_price = gas_price;
        self
    }

    pub fn invalid_transaction_ratio(mut self, invalid_transaction_ratio: usize) -> Self {
        self.invalid_transaction_ratio = invalid_transaction_ratio;
        self
    }

    pub fn transaction_type(mut self, transaction_type: TransactionType) -> Self {
        self.transaction_type = transaction_type;
        self
    }

    pub fn mode(mut self, mode: EmitJobMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn reuse_accounts(mut self) -> Self {
        self.reuse_accounts = true;
        self
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn txn_expiration_time_secs(mut self, txn_expiration_time_secs: u64) -> Self {
        self.txn_expiration_time_secs = txn_expiration_time_secs;
        self
    }

    pub fn check_stats_at_end(mut self, check_stats_at_end: bool) -> Self {
        self.check_stats_at_end = check_stats_at_end;
        self
    }

    pub fn calculate_mode_params(&self) -> EmitModeParams {
        let clients_count = self.rest_clients.len();

        match self.mode {
            EmitJobMode::MaxLoad { mempool_backlog } => {
                // The target mempool backlog is set to be 3x of the target TPS because of the on an average,
                // we can ~3 blocks in consensus queue. As long as we have 3x the target TPS as backlog,
                // it should be enough to produce the target TPS.
                let transactions_per_account = 5;
                let num_workers_per_endpoint = max(
                    mempool_backlog / (clients_count * transactions_per_account),
                    1,
                );

                info!(
                    " Transaction emitter target mempool backlog is {}",
                    mempool_backlog
                );

                info!(
                    " Will use {} clients and {} workers per client",
                    clients_count, num_workers_per_endpoint
                );

                EmitModeParams {
                    wait_millis: 0,
                    wait_committed: true,
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    check_stats_at_end: self.check_stats_at_end,
                    transactions_per_account,
                    max_submit_batch_size: 100,
                    start_offset_multiplier_millis: 0.0,
                    start_jitter_millis: 5000,
                    accounts_per_worker: 1,
                    workers_per_endpoint: num_workers_per_endpoint,
                    check_account_sequence_only_once: false,
                }
            }
            EmitJobMode::ConstTps { tps } => {
                let wait_seconds = self.txn_expiration_time_secs + 120;
                let batch_size = min(100, tps / clients_count);
                assert!(
                    batch_size > 0,
                    "TPS ({}) needs to be larger than clients_count ({})",
                    tps,
                    clients_count
                );

                // Actual TPS is clients_count * num_workers_per_endpoint * batch_size / (wait_millis / 1000), so:
                let num_workers_per_endpoint =
                    (tps * wait_seconds as usize) / clients_count / batch_size;
                assert!(
                    num_workers_per_endpoint > 0,
                    "Requested too small TPS: {}",
                    tps
                );

                info!(" Transaction emitter batch_size is {}", batch_size);

                info!(
                    " Will use {} clients and {} workers per client",
                    clients_count, num_workers_per_endpoint
                );

                EmitModeParams {
                    wait_millis: wait_seconds * 1000,
                    wait_committed: true,
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    check_stats_at_end: self.check_stats_at_end,
                    transactions_per_account: batch_size,
                    max_submit_batch_size: 100,
                    start_offset_multiplier_millis: (wait_seconds * 1000) as f64
                        / (num_workers_per_endpoint * clients_count) as f64,
                    start_jitter_millis: 5000,
                    accounts_per_worker: 1,
                    workers_per_endpoint: num_workers_per_endpoint,
                    check_account_sequence_only_once: true,
                }
            }
        }
    }
}

#[derive(Debug)]
struct Worker {
    join_handle: JoinHandle<Vec<LocalAccount>>,
}

#[derive(Debug)]
pub struct EmitJob {
    workers: Vec<Worker>,
    stop: Arc<AtomicBool>,
    stats: Arc<StatsAccumulator>,
}

#[derive(Debug)]
pub struct TxnEmitter<'t> {
    accounts: Vec<LocalAccount>,
    txn_factory: TransactionFactory,
    client: RestClient,
    rng: StdRng,
    root_account: &'t mut LocalAccount,
}

impl<'t> TxnEmitter<'t> {
    pub fn new(
        root_account: &'t mut LocalAccount,
        client: RestClient,
        transaction_factory: TransactionFactory,
        rng: StdRng,
    ) -> Self {
        Self {
            accounts: vec![],
            txn_factory: transaction_factory,
            root_account,
            client,
            rng,
        }
    }

    pub fn take_account(&mut self) -> LocalAccount {
        self.accounts.remove(0)
    }

    pub fn clear(&mut self) {
        self.accounts.clear();
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn from_rng(&mut self) -> StdRng {
        StdRng::from_rng(self.rng()).unwrap()
    }

    pub async fn get_money_source(&mut self, coins_total: u64) -> Result<&mut LocalAccount> {
        let client = self.client.clone();
        info!("Creating and minting faucet account");
        let faucet_account = &mut self.root_account;
        let balance = client
            .get_account_balance(faucet_account.address())
            .await?
            .into_inner();
        info!(
            "Root account current balances are {}, requested {} coins",
            balance.get(),
            coins_total
        );
        Ok(faucet_account)
    }

    pub async fn start_job(&mut self, req: EmitJobRequest) -> Result<EmitJob> {
        let mode_params = req.calculate_mode_params();
        let workers_per_endpoint = mode_params.workers_per_endpoint;
        let num_workers = req.rest_clients.len() * workers_per_endpoint;
        let num_accounts = num_workers * mode_params.accounts_per_worker;
        info!(
            "Will use {} workers per endpoint for a total of {} endpoint clients and {} accounts",
            workers_per_endpoint, num_workers, num_accounts
        );
        info!("Will create a total of {} accounts", num_accounts);
        let mut account_minter = AccountMinter::new(
            self.root_account,
            self.txn_factory.clone(),
            self.rng.clone(),
        );
        let mut new_accounts = account_minter
            .mint_accounts(&req, &mode_params, num_accounts)
            .await?;
        self.accounts.append(&mut new_accounts);
        let all_accounts = self.accounts.split_off(self.accounts.len() - num_accounts);
        let all_addresses: Vec<_> = all_accounts.iter().map(|d| d.address()).collect();
        let all_addresses = Arc::new(all_addresses);
        let mut all_accounts = all_accounts.into_iter();
        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(StatsAccumulator::default());
        let tokio_handle = Handle::current();
        let txn_factory = self
            .txn_factory
            .clone()
            .with_transaction_expiration_time(mode_params.txn_expiration_time_secs);
        let txn_generator_creator: Box<dyn TransactionGeneratorCreator> = match req.transaction_type
        {
            TransactionType::P2P => Box::new(P2PTransactionGeneratorCreator::new(
                self.from_rng(),
                txn_factory,
                SEND_AMOUNT,
            )),
            TransactionType::AccountGeneration => {
                Box::new(AccountGeneratorCreator::new(txn_factory))
            }
            TransactionType::NftMint => Box::new(
                NFTMintGeneratorCreator::new(
                    self.from_rng(),
                    txn_factory,
                    self.root_account,
                    req.rest_clients[0].clone(),
                )
                .await,
            ),
        };
        let mut workers = vec![];
        for client in req.rest_clients {
            for _ in 0..workers_per_endpoint {
                let accounts = (&mut all_accounts)
                    .take(mode_params.accounts_per_worker)
                    .collect();
                let all_addresses = all_addresses.clone();
                let stop = stop.clone();
                let stats = Arc::clone(&stats);

                let worker = SubmissionWorker::new(
                    accounts,
                    client.clone(),
                    all_addresses,
                    stop,
                    mode_params.clone(),
                    stats,
                    txn_generator_creator.create_transaction_generator(),
                    req.invalid_transaction_ratio,
                    self.from_rng(),
                    workers.len(),
                );
                let join_handle = tokio_handle.spawn(worker.run(req.gas_price).boxed());
                workers.push(Worker { join_handle });
            }
        }
        info!("Tx emitter workers started");
        Ok(EmitJob {
            workers,
            stop,
            stats,
        })
    }

    pub async fn stop_job(&mut self, job: EmitJob) -> TxnStats {
        job.stop.store(true, Ordering::Relaxed);
        for worker in job.workers {
            let mut accounts = worker
                .join_handle
                .await
                .expect("TxnEmitter worker thread failed");
            self.accounts.append(&mut accounts);
        }
        job.stats.accumulate()
    }

    pub fn peek_job_stats(&self, job: &EmitJob) -> TxnStats {
        job.stats.accumulate()
    }

    pub async fn periodic_stat(&mut self, job: &EmitJob, duration: Duration, interval_secs: u64) {
        let deadline = Instant::now() + duration;
        let mut prev_stats: Option<TxnStats> = None;
        let window = Duration::from_secs(min(interval_secs, 1));
        while Instant::now() < deadline {
            tokio::time::sleep(window).await;
            let stats = self.peek_job_stats(job);
            let delta = &stats - &prev_stats.unwrap_or_default();
            prev_stats = Some(stats);
            info!("{}", delta.rate(window));
        }
    }

    pub async fn emit_txn_for(&mut self, emit_job_request: EmitJobRequest) -> Result<TxnStats> {
        let duration = emit_job_request.duration;
        let job = self.start_job(emit_job_request).await?;
        info!("Starting emitting txns for {} secs", duration.as_secs());
        time::sleep(duration).await;
        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let stats = self.stop_job(job).await;
        info!("Stopped job");
        Ok(stats)
    }

    pub async fn emit_txn_for_with_stats(
        &mut self,
        emit_job_request: EmitJobRequest,
        interval_secs: u64,
    ) -> Result<TxnStats> {
        let duration = emit_job_request.duration;
        info!("Starting emitting txns for {} secs", duration.as_secs());
        let job = self.start_job(emit_job_request).await?;
        self.periodic_stat(&job, duration, interval_secs).await;
        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let stats = self.stop_job(job).await;
        info!("Stopped job");
        Ok(stats)
    }

    pub async fn submit_single_transaction(
        &self,
        client: &RestClient,
        sender: &mut LocalAccount,
        receiver: &AccountAddress,
        num_coins: u64,
    ) -> Result<Instant> {
        let txn = gen_transfer_txn_request(sender, receiver, num_coins, &self.txn_factory, 1);
        client.submit(&txn).await?;
        let deadline = Instant::now() + Duration::from_secs(txn.expiration_timestamp_secs() + 30);
        Ok(deadline)
    }
}

/// Waits for a single account to catch up to the expected sequence number
async fn wait_for_single_account_sequence(
    client: &RestClient,
    account: &LocalAccount,
    wait_timeout: Duration,
) -> Result<()> {
    let deadline = Instant::now() + wait_timeout;
    while Instant::now() <= deadline {
        time::sleep(Duration::from_millis(1000)).await;
        match query_sequence_numbers(client, [account.address()].iter()).await {
            Ok(sequence_numbers) => {
                if sequence_numbers[0] >= account.sequence_number() {
                    return Ok(());
                }
            }
            Err(e) => {
                info!(
                    "Failed to query sequence number for account {:?} for instance {:?} : {:?}",
                    account, client, e
                );
            }
        }
    }
    Err(anyhow!(
        "Timed out waiting for single account {:?} sequence number for instance {:?}",
        account,
        client
    ))
}

/// This function waits for the submitted transactions to be committed, up to
/// a deadline. If some accounts still have uncommitted transactions when we
/// hit the deadline, we return a map of account to the info about the number
/// of committed transactions, based on the delta between the local sequence
/// number and the actual sequence number returned by the account. Note, this
/// can return possibly unexpected results if the emitter was emitting more
/// transactions per account than the mempool limit of the accounts on the node.
/// As it is now, the sequence number of the local account incrememnts regardless
/// of whether the transaction is accepted into the node's mempool or not. So the
/// local sequence number could be much higher than the real sequence number ever
/// will be, since not all of the submitted transactions were accepted.
/// TODO, investigate whether this behaviour is desirable.
async fn wait_for_accounts_sequence(
    start_time: Instant,
    client: &RestClient,
    accounts: &mut [LocalAccount],
    transactions_per_account: usize,
    wait_timeout: Duration,
    fetch_only_once: bool,
    rng: &mut StdRng,
) -> Result<(), HashMap<AccountAddress, usize>> {
    let deadline = start_time + wait_timeout;
    let mut pending_addresses: HashSet<_> = accounts.iter().map(|d| d.address()).collect();
    let mut latest_fetched_counts = HashMap::new();

    if !fetch_only_once {
        // Choose a random account and wait for its sequence number to be up to date. After that, we can
        // query the all the accounts. This will help us ensure we don't hammer the REST API with too many
        // query for all the accounts.
        let account = accounts.choose(rng).expect("accounts can't be empty");
        if wait_for_single_account_sequence(client, account, wait_timeout)
            .await
            .is_err()
        {
            return failed_transaction_counts_result(
                accounts,
                transactions_per_account,
                latest_fetched_counts,
            );
        }
    }

    // Special case for single account
    if accounts.len() == 1 {
        return Ok(());
    }

    while Instant::now() <= deadline {
        match query_sequence_numbers(client, pending_addresses.iter()).await {
            Ok(sequence_numbers) => {
                for (account, sequence_number) in zip(accounts.iter_mut(), &sequence_numbers) {
                    latest_fetched_counts.insert(account.address(), *sequence_number);

                    if account.sequence_number() == *sequence_number || fetch_only_once {
                        pending_addresses.remove(&account.address());
                    }
                }

                if pending_addresses.is_empty() {
                    break;
                }
            }
            Err(e) => {
                info!(
                    "Failed to query ledger info on accounts {:?} for instance {:?} : {:?}",
                    pending_addresses, client, e
                );
            }
        }
        time::sleep(Duration::from_millis(1000)).await;
    }

    failed_transaction_counts_result(accounts, transactions_per_account, latest_fetched_counts)
}

fn failed_transaction_counts_result(
    accounts: &mut [LocalAccount],
    transactions_per_account: usize,
    latest_fetched_counts: HashMap<AccountAddress, u64>,
) -> Result<(), HashMap<AccountAddress, usize>> {
    let result = accounts
        .iter_mut()
        .map(
            |account| match latest_fetched_counts.get(&account.address()) {
                Some(count) => {
                    assert!(account.sequence_number() > *count);
                    assert!(account.sequence_number() <= count + transactions_per_account as u64);
                    let diff = (account.sequence_number() - count) as usize;
                    *account.sequence_number_mut() = *count;
                    (account.address(), diff)
                }
                None => {
                    *account.sequence_number_mut() -= transactions_per_account as u64;
                    (account.address(), transactions_per_account)
                }
            },
        )
        .collect::<HashMap<_, _>>();

    if result.is_empty() {
        Ok(())
    } else {
        Err(result)
    }
}

pub async fn query_sequence_numbers<'a, I>(client: &RestClient, addresses: I) -> Result<Vec<u64>>
where
    I: Iterator<Item = &'a AccountAddress>,
{
    Ok(try_join_all(
        addresses.map(|address| RETRY_POLICY.retry(move || client.get_account(*address))),
    )
    .await
    .map_err(|e| format_err!("Get accounts failed: {}", e))?
    .into_iter()
    .map(|resp| resp.into_inner().sequence_number)
    .collect())
}

pub fn gen_transfer_txn_request(
    sender: &mut LocalAccount,
    receiver: &AccountAddress,
    num_coins: u64,
    txn_factory: &TransactionFactory,
    gas_price: u64,
) -> SignedTransaction {
    sender.sign_with_transaction_builder(
        txn_factory
            .payload(aptos_stdlib::aptos_coin_transfer(*receiver, num_coins))
            .gas_unit_price(gas_price),
    )
}
