// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod account_minter;
pub mod stats;
pub mod submission_worker;

use again::RetryPolicy;
use anyhow::{anyhow, format_err, Result};
use aptos_infallible::RwLock;
use aptos_logger::sample::Sampling;
use aptos_logger::{debug, error, info, sample, sample::SampleRate, warn};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use futures::future::{try_join_all, FutureExt};
use itertools::zip;
use once_cell::sync::Lazy;
use rand::seq::IteratorRandom;
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
    emitter::{
        account_minter::AccountMinter,
        stats::{DynamicStatsTracking, TxnStats},
        submission_worker::SubmissionWorker,
    },
    transaction_generator::{
        account_generator::AccountGeneratorCreator, nft_mint::NFTMintGeneratorCreator,
        p2p_transaction_generator::P2PTransactionGeneratorCreator,
        transaction_mix_generator::TxnMixGeneratorCreator, TransactionGeneratorCreator,
    },
};
use aptos_sdk::transaction_builder::aptos_stdlib;
use rand::rngs::StdRng;

// Max is 100k TPS for a full day.
const MAX_TXNS: u64 = 100_000_000_000;
const SEND_AMOUNT: u64 = 1;
const GAS_AMOUNT: u64 = aptos_global_constants::MAX_GAS_AMOUNT;

// This retry policy is used for important client calls necessary for setting
// up the test (e.g. account creation) and collecting its results (e.g. checking
// account sequence numbers). If these fail, the whole test fails. We do not use
// this for submitting transactions, as we have a way to handle when that fails.
// This retry policy means an operation will take 8 seconds at most.
pub static RETRY_POLICY: Lazy<RetryPolicy> = Lazy::new(|| {
    RetryPolicy::exponential(Duration::from_millis(125))
        .with_max_retries(6)
        .with_jitter(true)
});

#[derive(Clone, Debug)]
pub struct EmitModeParams {
    pub txn_expiration_time_secs: u64,

    pub workers_per_endpoint: usize,
    pub accounts_per_worker: usize,

    /// Max transactions per account in mempool
    pub transactions_per_account: usize,
    pub max_submit_batch_size: usize,
    pub start_offset_multiplier_millis: f64,
    pub start_jitter_millis: u64,
    pub wait_millis: u64,
    pub check_account_sequence_only_once_fraction: f32,
    pub check_account_sequence_sleep_millis: u64,
}

#[derive(Clone, Debug)]
pub enum EmitJobMode {
    MaxLoad { mempool_backlog: usize },
    ConstTps { tps: usize },
}

impl EmitJobMode {
    pub fn create(mempool_backlog: Option<usize>, target_tps: Option<usize>) -> Self {
        if let Some(mempool_backlog_val) = mempool_backlog {
            assert!(
                target_tps.is_none(),
                "Cannot set both mempool_backlog and target_tps"
            );
            Self::MaxLoad {
                mempool_backlog: mempool_backlog_val,
            }
        } else {
            Self::ConstTps {
                tps: target_tps.expect("Need to set either mempool_backlog or target_tps"),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct EmitJobRequest {
    rest_clients: Vec<RestClient>,
    mode: EmitJobMode,

    gas_price: u64,
    invalid_transaction_ratio: usize,
    reuse_accounts: bool,
    mint_to_root: bool,

    transaction_mix: Vec<(TransactionType, usize)>,

    add_created_accounts_to_pool: bool,
    max_account_working_set: usize,

    txn_expiration_time_secs: u64,
}

impl Default for EmitJobRequest {
    fn default() -> Self {
        Self {
            rest_clients: Vec::new(),
            mode: EmitJobMode::MaxLoad {
                mempool_backlog: 3000,
            },
            gas_price: aptos_global_constants::GAS_UNIT_PRICE,
            invalid_transaction_ratio: 0,
            reuse_accounts: false,
            mint_to_root: false,
            transaction_mix: vec![(TransactionType::P2P, 1)],
            add_created_accounts_to_pool: true,
            max_account_working_set: 1_000_000,
            txn_expiration_time_secs: 60,
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
        self.transaction_mix = vec![(transaction_type, 1)];
        self
    }

    pub fn transaction_mix(mut self, transaction_mix: Vec<(TransactionType, usize)>) -> Self {
        self.transaction_mix = transaction_mix;
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

    pub fn add_created_accounts_to_pool(mut self, add_created_accounts_to_pool: bool) -> Self {
        self.add_created_accounts_to_pool = add_created_accounts_to_pool;
        self
    }

    pub fn max_account_working_set(mut self, max_account_working_set: usize) -> Self {
        self.max_account_working_set = max_account_working_set;
        self
    }

    pub fn txn_expiration_time_secs(mut self, txn_expiration_time_secs: u64) -> Self {
        self.txn_expiration_time_secs = txn_expiration_time_secs;
        self
    }

    pub fn calculate_mode_params(&self) -> EmitModeParams {
        let clients_count = self.rest_clients.len();

        match self.mode {
            EmitJobMode::MaxLoad { mempool_backlog } => {
                // The target mempool backlog is set to be 3x of the target TPS because of the on an average,
                // we can ~3 blocks in consensus queue. As long as we have 3x the target TPS as backlog,
                // it should be enough to produce the target TPS.
                let transactions_per_account = 20;
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
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    transactions_per_account,
                    max_submit_batch_size: 100,
                    start_offset_multiplier_millis: 0.0,
                    start_jitter_millis: 5000,
                    accounts_per_worker: 1,
                    workers_per_endpoint: num_workers_per_endpoint,
                    check_account_sequence_only_once_fraction: 0.0,
                    check_account_sequence_sleep_millis: 300,
                }
            }
            EmitJobMode::ConstTps { tps } => {
                // We are going to create ConstTps (open-loop) txn-emitter, by:
                // - having a single worker handle a single account, with:
                //   - issuing a batch request (which generally either suceeeds or fails)
                //   - waits for transaction expiration
                //   - issues a single call to get updated sequence_number, to know how many
                //     transactions succeeded
                //   - wait until our time.
                // If we always finish first 3 steps before our time, we have a constant TPS of:
                // clients_count * num_workers_per_endpoint * transactions_per_account / (wait_millis / 1000)
                // Also, with transactions_per_account = 100, only 1% of the load should be coming from fetching
                // sequence number from the account, so that it doesn't affect the TPS meaningfully.
                //
                // That's why we set wait_seconds conservativelly, to make sure all processing and
                // client calls finish within that time.

                let wait_seconds = self.txn_expiration_time_secs + 180;
                // In case we set a very low TPS, we need to still be able to spread out
                // transactions, at least to the seconds granularity, so we reduce transactions_per_account
                // if needed.
                let transactions_per_account = min(100, tps);
                assert!(
                    transactions_per_account > 0,
                    "TPS ({}) needs to be larger than 0",
                    tps,
                );

                // compute num_workers_per_endpoint, so that target_tps is achieved.
                let num_workers_per_endpoint =
                    (tps * wait_seconds as usize) / clients_count / transactions_per_account;
                assert!(
                    num_workers_per_endpoint > 0,
                    "Requested too small TPS: {}",
                    tps
                );

                info!(
                    " Transaction emitter targetting {} TPS, expecting {} TPS",
                    tps,
                    clients_count * num_workers_per_endpoint * transactions_per_account
                        / wait_seconds as usize
                );

                info!(
                    " Transaction emitter transactions_per_account batch is {}, with wait_seconds {}",
                    transactions_per_account, wait_seconds
                );

                // sample latency on 2% of requests, or at least once every 5s.
                let sample_latency_fraction = 1.0_f32.min(0.02_f32.max(
                    wait_seconds as f32
                        / (clients_count * num_workers_per_endpoint) as f32
                        / 5.0_f32,
                ));

                info!(
                    " Will use {} clients and {} workers per client, sampling latency on {}",
                    clients_count, num_workers_per_endpoint, sample_latency_fraction
                );

                EmitModeParams {
                    wait_millis: wait_seconds * 1000,
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    transactions_per_account,
                    max_submit_batch_size: 100,
                    start_offset_multiplier_millis: (wait_seconds * 1000) as f64
                        / (num_workers_per_endpoint * clients_count) as f64,
                    // Using jitter here doesn't make TPS vary enough, as we have many workers.
                    // If we wanted to support that, we could for example incrementally vary the offset.
                    start_jitter_millis: 0,
                    accounts_per_worker: 1,
                    workers_per_endpoint: num_workers_per_endpoint,
                    check_account_sequence_only_once_fraction: 1.0 - sample_latency_fraction,
                    check_account_sequence_sleep_millis: 300,
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
    stats: Arc<DynamicStatsTracking>,
}

impl EmitJob {
    pub fn start_next_phase(&self) {
        self.stats.start_next_phase();
    }

    pub fn get_cur_phase(&self) -> usize {
        self.stats.get_cur_phase()
    }
}

#[derive(Debug)]
pub struct TxnEmitter {
    accounts: Vec<LocalAccount>,
    txn_factory: TransactionFactory,
    rng: StdRng,
}

impl TxnEmitter {
    pub fn new(transaction_factory: TransactionFactory, rng: StdRng) -> Self {
        Self {
            accounts: vec![],
            txn_factory: transaction_factory,
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

    pub async fn start_job(
        &mut self,
        root_account: &mut LocalAccount,
        req: EmitJobRequest,
        stats_tracking_phases: usize,
    ) -> Result<EmitJob> {
        let mode_params = req.calculate_mode_params();
        let workers_per_endpoint = mode_params.workers_per_endpoint;
        let num_workers = req.rest_clients.len() * workers_per_endpoint;
        let num_accounts = num_workers * mode_params.accounts_per_worker;
        info!(
            "Will use {} workers per endpoint for a total of {} endpoint clients and {} accounts",
            workers_per_endpoint, num_workers, num_accounts
        );
        let mut account_minter =
            AccountMinter::new(root_account, self.txn_factory.clone(), self.rng.clone());
        let mut new_accounts = account_minter
            .create_accounts(&req, &mode_params, num_accounts)
            .await?;
        self.accounts.append(&mut new_accounts);
        let all_accounts = self.accounts.split_off(self.accounts.len() - num_accounts);
        let all_addresses: Vec<_> = all_accounts.iter().map(|d| d.address()).collect();
        let all_addresses = Arc::new(RwLock::new(all_addresses));
        let mut all_accounts = all_accounts.into_iter();
        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(DynamicStatsTracking::new(stats_tracking_phases));
        let tokio_handle = Handle::current();
        let txn_factory = self
            .txn_factory
            .clone()
            .with_transaction_expiration_time(mode_params.txn_expiration_time_secs);
        let mut txn_generator_creator_mix: Vec<(Box<dyn TransactionGeneratorCreator>, usize)> =
            Vec::new();
        for (transaction_type, weight) in req.transaction_mix {
            let txn_generator_creator: Box<dyn TransactionGeneratorCreator> = match transaction_type
            {
                TransactionType::P2P => Box::new(P2PTransactionGeneratorCreator::new(
                    self.from_rng(),
                    txn_factory.clone(),
                    SEND_AMOUNT,
                    all_addresses.clone(),
                    req.invalid_transaction_ratio,
                    req.gas_price,
                )),
                TransactionType::AccountGeneration => Box::new(AccountGeneratorCreator::new(
                    txn_factory.clone(),
                    all_addresses.clone(),
                    req.add_created_accounts_to_pool,
                    req.max_account_working_set,
                    req.gas_price,
                )),
                TransactionType::NftMint => Box::new(
                    NFTMintGeneratorCreator::new(
                        self.from_rng(),
                        txn_factory.clone(),
                        root_account,
                        req.rest_clients[0].clone(),
                    )
                    .await,
                ),
            };
            txn_generator_creator_mix.push((txn_generator_creator, weight));
        }
        let txn_generator_creator: Box<dyn TransactionGeneratorCreator> =
            if txn_generator_creator_mix.len() > 1 {
                Box::new(TxnMixGeneratorCreator::new(txn_generator_creator_mix))
            } else {
                txn_generator_creator_mix.into_iter().next().unwrap().0
            };

        let total_workers = req.rest_clients.len() * workers_per_endpoint;

        let check_account_sequence_only_once_for = (0..total_workers)
            .choose_multiple(
                &mut self.from_rng(),
                (mode_params.check_account_sequence_only_once_fraction * total_workers as f32)
                    as usize,
            )
            .into_iter()
            .collect::<HashSet<_>>();

        info!(
            "Checking account sequence and counting latency for {} out of {} total_workers",
            total_workers - check_account_sequence_only_once_for.len(),
            total_workers
        );

        let mut workers = vec![];
        for _ in 0..workers_per_endpoint {
            for client in &req.rest_clients {
                let accounts = (&mut all_accounts)
                    .take(mode_params.accounts_per_worker)
                    .collect();
                let stop = stop.clone();
                let stats = Arc::clone(&stats);

                let worker = SubmissionWorker::new(
                    accounts,
                    client.clone(),
                    stop,
                    mode_params.clone(),
                    stats,
                    txn_generator_creator.create_transaction_generator(),
                    workers.len(),
                    check_account_sequence_only_once_for.contains(&workers.len()),
                    self.from_rng(),
                );
                let join_handle = tokio_handle.spawn(worker.run().boxed());
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

    pub async fn stop_job(&mut self, job: EmitJob) -> Vec<TxnStats> {
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

    pub fn peek_job_stats(&self, job: &EmitJob) -> Vec<TxnStats> {
        job.stats.accumulate()
    }

    pub async fn periodic_stat(&mut self, job: &EmitJob, duration: Duration, interval_secs: u64) {
        let deadline = Instant::now() + duration;
        let mut prev_stats: Option<Vec<TxnStats>> = None;
        let default_stats = TxnStats::default();
        let window = Duration::from_secs(max(interval_secs, 1));
        while Instant::now() < deadline {
            tokio::time::sleep(window).await;
            let cur_phase = job.stats.get_cur_phase();
            let stats = self.peek_job_stats(job);
            let delta = &stats[cur_phase]
                - prev_stats
                    .as_ref()
                    .map(|p| &p[cur_phase])
                    .unwrap_or(&default_stats);
            prev_stats = Some(stats);
            info!("phase {}: {}", cur_phase, delta.rate(window));
        }
    }

    pub async fn emit_txn_for(
        &mut self,
        root_account: &mut LocalAccount,
        emit_job_request: EmitJobRequest,
        duration: Duration,
    ) -> Result<TxnStats> {
        let job = self.start_job(root_account, emit_job_request, 1).await?;
        info!("Starting emitting txns for {} secs", duration.as_secs());
        time::sleep(duration).await;
        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let stats = self.stop_job(job).await;
        info!("Stopped job");
        Ok(stats.into_iter().next().unwrap())
    }

    pub async fn emit_txn_for_with_stats(
        &mut self,
        root_account: &mut LocalAccount,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        interval_secs: u64,
    ) -> Result<TxnStats> {
        info!("Starting emitting txns for {} secs", duration.as_secs());
        let job = self.start_job(root_account, emit_job_request, 1).await?;
        self.periodic_stat(&job, duration, interval_secs).await;
        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let stats = self.stop_job(job).await;
        info!("Stopped job");
        Ok(stats.into_iter().next().unwrap())
    }

    pub async fn submit_single_transaction(
        &self,
        client: &RestClient,
        sender: &mut LocalAccount,
        receiver: &AccountAddress,
        num_coins: u64,
    ) -> Result<Instant> {
        let txn = gen_transfer_txn_request(sender, receiver, num_coins, &self.txn_factory);
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
        match query_sequence_number(client, account.address()).await {
            Ok(sequence_number) => {
                if sequence_number >= account.sequence_number() {
                    return Ok(());
                }
            }
            Err(e) => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(60)),
                    warn!(
                        "Failed to query sequence number for account {:?} for instance {:?} : {:?}",
                        account, client, e
                    )
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
/// a wait_timeout (counted from the start_time passed in, not from the function call).
/// It returns number of transactions that expired without being committed,
/// and sum of completion timestamps for those that have.
///
/// This function updates sequence_number for the account to match what
/// we were able to fetch last.
async fn wait_for_accounts_sequence(
    start_time: Instant,
    client: &RestClient,
    accounts: &mut [LocalAccount],
    transactions_per_account: usize,
    txn_expiration_ts_secs: u64,
    sleep_between_cycles: Duration,
) -> (usize, u128) {
    let mut pending_addresses: HashSet<_> = accounts.iter().map(|d| d.address()).collect();
    let mut latest_fetched_counts = HashMap::new();

    let mut sum_of_completion_timestamps_millis = 0u128;
    loop {
        match query_sequence_numbers(client, pending_addresses.iter()).await {
            Ok((sequence_numbers, ledger_timestamp_secs)) => {
                let millis_elapsed = start_time.elapsed().as_millis();
                for (account, sequence_number) in zip(accounts.iter_mut(), &sequence_numbers) {
                    let prev_sequence_number = latest_fetched_counts
                        .insert(account.address(), *sequence_number)
                        .unwrap_or(account.sequence_number() - transactions_per_account as u64);
                    assert!(prev_sequence_number <= *sequence_number);
                    sum_of_completion_timestamps_millis +=
                        millis_elapsed * (*sequence_number - prev_sequence_number) as u128;

                    if account.sequence_number() == *sequence_number {
                        pending_addresses.remove(&account.address());
                    }
                }

                if pending_addresses.is_empty() {
                    break;
                }

                if ledger_timestamp_secs > txn_expiration_ts_secs {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(60)),
                        warn!(
                            "[{}] Ledger timestamp {} exceeded txn expiration timestamp {} for {:?}",
                            client.path_prefix_string(),
                            ledger_timestamp_secs,
                            txn_expiration_ts_secs,
                            accounts
                                .iter()
                                .map(|a| a.address().to_hex_literal())
                                .collect::<Vec<_>>(),
                        )
                    );
                    break;
                }
            }
            Err(e) => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(60)),
                    warn!(
                        "[{}] Failed to query ledger info on accounts {:?}: {:?}",
                        client.path_prefix_string(),
                        pending_addresses,
                        e
                    )
                );
            }
        }

        if aptos_infallible::duration_since_epoch().as_secs() >= txn_expiration_ts_secs + 240 {
            sample!(
                SampleRate::Duration(Duration::from_secs(15)),
                error!(
                    "[{}] Client cannot catch up to needed timestamp ({}), after additional 240s, aborting",
                    client.path_prefix_string(),
                    txn_expiration_ts_secs,
                )
            );
            break;
        }

        time::sleep(sleep_between_cycles).await;
    }

    (
        update_seq_num_and_get_num_expired(
            accounts,
            transactions_per_account,
            latest_fetched_counts,
        ),
        sum_of_completion_timestamps_millis,
    )
}

fn update_seq_num_and_get_num_expired(
    accounts: &mut [LocalAccount],
    transactions_per_account: usize,
    latest_fetched_counts: HashMap<AccountAddress, u64>,
) -> usize {
    accounts
        .iter_mut()
        .filter_map(
            |account| match latest_fetched_counts.get(&account.address()) {
                Some(count) => {
                    if *count != account.sequence_number() {
                        assert!(account.sequence_number() > *count);
                        assert!(
                            account.sequence_number() <= count + transactions_per_account as u64
                        );
                        let diff = (account.sequence_number() - count) as usize;
                        debug!(
                            "Stale sequence_number for {}, expected {}, setting to {}",
                            account.address(),
                            account.sequence_number(),
                            count
                        );
                        *account.sequence_number_mut() = *count;
                        Some(diff)
                    } else {
                        None
                    }
                }
                None => {
                    debug!(
                        "Couldn't fetch sequence_number for {}, expected {}, setting to {}",
                        account.address(),
                        account.sequence_number(),
                        account.sequence_number() - transactions_per_account as u64
                    );
                    *account.sequence_number_mut() -= transactions_per_account as u64;
                    Some(transactions_per_account)
                }
            },
        )
        .sum()
}

pub async fn query_sequence_number(client: &RestClient, address: AccountAddress) -> Result<u64> {
    Ok(query_sequence_numbers(client, [address].iter()).await?.0[0])
}

// Return a pair of (list of sequence numbers, ledger timestamp)
pub async fn query_sequence_numbers<'a, I>(
    client: &RestClient,
    addresses: I,
) -> Result<(Vec<u64>, u64)>
where
    I: Iterator<Item = &'a AccountAddress>,
{
    let (seq_nums, timestamps): (Vec<_>, Vec<_>) = try_join_all(
        addresses.map(|address| RETRY_POLICY.retry(move || client.get_account_bcs(*address))),
    )
    .await
    .map_err(|e| format_err!("Get accounts failed: {:?}", e))?
    .into_iter()
    .map(|resp| {
        let (account, state) = resp.into_parts();
        (
            account.sequence_number(),
            Duration::from_micros(state.timestamp_usecs).as_secs(),
        )
    })
    .unzip();

    // return min for the timestamp, to make sure
    // all sequence numbers were <= to return values at that timestamp
    Ok((seq_nums, timestamps.into_iter().min().unwrap()))
}

pub fn gen_transfer_txn_request(
    sender: &mut LocalAccount,
    receiver: &AccountAddress,
    num_coins: u64,
    txn_factory: &TransactionFactory,
) -> SignedTransaction {
    sender.sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::aptos_coin_transfer(*receiver, num_coins)),
    )
}
