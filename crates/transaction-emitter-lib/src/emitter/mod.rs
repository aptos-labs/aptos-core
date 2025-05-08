// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod account_minter;
pub mod local_account_generator;
pub mod stats;
pub mod submission_worker;
pub mod transaction_executor;

use crate::emitter::{
    account_minter::{bulk_create_accounts, SourceAccountManager},
    local_account_generator::{
        create_keyless_account_generator, create_private_key_account_generator,
    },
    stats::{DynamicStatsTracking, TxnStats},
    submission_worker::SubmissionWorker,
    transaction_executor::RestApiReliableTransactionSubmitter,
};
use again::RetryPolicy;
use anyhow::{ensure, format_err, Result};
use aptos_config::config::DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_logger::{sample, sample::SampleRate};
use aptos_rest_client::{aptos_api_types::AptosErrorCode, error::RestError, Client as RestClient};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{transaction::SignedTransaction, AccountKey, LocalAccount},
};
use aptos_transaction_generator_lib::{
    create_txn_generator_creator, AccountType, TransactionType, SEND_AMOUNT,
};
use aptos_types::account_config::aptos_test_root_address;
use futures::future::{try_join_all, FutureExt};
use log::{error, info, warn};
use once_cell::sync::Lazy;
use rand::{
    rngs::StdRng,
    seq::{IteratorRandom, SliceRandom},
    Rng,
};
use rand_core::SeedableRng;
use std::{
    cmp::{max, min},
    collections::{HashMap, HashSet},
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{runtime::Handle, task::JoinHandle, time};

// Max is 100k TPS for 3 hours
const MAX_TXNS: u64 = 1_000_000_000;

// TODO Transfer cost increases during Coin => FA migration, we can reduce back later.
pub const EXPECTED_GAS_PER_TRANSFER: u64 = 50;
pub const EXPECTED_GAS_PER_ACCOUNT_CREATE: u64 = 1100 + 20;

const MAX_RETRIES: usize = 12;

// This retry policy is used for querying sequence numbers and account balances in the initialization step.
// If these fail, the whole test fails. Backoff is large, as generally only other side
// throttling our requests is the cause for failures.
// We do not use this for submitting transactions, as we have a way to handle when that fails.
static FETCH_ACCOUNT_RETRY_POLICY: Lazy<RetryPolicy> = Lazy::new(|| {
    RetryPolicy::exponential(Duration::from_secs(1))
        .with_max_retries(MAX_RETRIES)
        .with_jitter(true)
});

#[derive(Clone, Debug)]
pub struct EmitModeParams {
    pub txn_expiration_time_secs: u64,

    pub endpoints: usize,
    pub num_accounts: usize,
    /// Max transactions per account in mempool
    pub transactions_per_account: usize,
    pub max_submit_batch_size: usize,
    pub worker_offset_mode: WorkerOffsetMode,
    pub wait_millis: u64,
    pub check_account_sequence_only_once_fraction: f32,
    pub check_account_sequence_sleep: Duration,
}

#[derive(Clone, Debug)]
pub enum WorkerOffsetMode {
    NoOffset,
    Jitter { jitter_millis: u64 },
    Spread,
    Wave { wave_ratio: f64, num_waves: f64 },
}

#[derive(Clone, Debug)]
pub enum EmitJobMode {
    MaxLoad {
        mempool_backlog: usize,
    },
    ConstTps {
        tps: usize,
    },
    WaveTps {
        average_tps: usize,
        // amount of traffic that is oscilating:
        // 1.0 means it oscilates between [0, 2 * average_tps]
        // 0.3 means it oscilates between [0.7 * average_tps, 1.3 * average_tps]
        wave_ratio: f32,
        // number of waves within the wait_millis interval (which is txn_expiration_time + 180s)
        num_waves: usize,
    },
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
pub enum NumAccountsMode {
    NumAccounts(usize),
    TransactionsPerAccount(usize),
}

impl NumAccountsMode {
    pub fn create(num_accounts: Option<usize>, transactions_per_account: Option<usize>) -> Self {
        match (num_accounts, transactions_per_account) {
            (Some(num_accounts), None) => Self::NumAccounts(num_accounts),
            (None, Some(transactions_per_account)) => {
                Self::TransactionsPerAccount(transactions_per_account)
            },
            _ => panic!(
                "Either num_accounts or transactions_per_account should be set, but not both"
            ),
        }
    }
}

/// total coins consumed are less than 2 * max_txns * expected_gas_per_txn * gas_price,
/// which is by default 100000000000 * 100000, but can be overriden.
#[derive(Clone, Debug)]
pub struct EmitJobRequest {
    rest_clients: Vec<RestClient>,
    mode: EmitJobMode,

    transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,

    max_gas_per_txn: u64,
    init_max_gas_per_txn: Option<u64>,

    expected_max_txns: u64,

    expected_gas_per_txn: Option<u64>,
    expected_gas_per_transfer: u64,
    expected_gas_per_account_create: u64,

    coins_per_account_override: Option<u64>,

    gas_price: u64,
    init_gas_price_multiplier: u64,

    mint_to_root: bool,
    skip_funding_accounts: bool,

    txn_expiration_time_secs: u64,
    init_expiration_multiplier: f64,

    init_retry_interval: Duration,
    num_accounts_mode: NumAccountsMode,
    prompt_before_spending: bool,

    coordination_delay_between_instances: Duration,

    latency_polling_interval: Duration,
    // Default additional wait is (txn_expiration_time_secs + 5). Override to wait for different length.
    tps_wait_after_expiration_secs: Option<u64>,

    account_minter_seed: Option<[u8; 32]>,

    account_type: AccountType,

    // Arguments for Keyless Load Testing
    keyless_ephem_secret_key: Option<[u8; 32]>,

    proof_file_path: Option<String>,

    epk_expiry_date_secs: Option<u64>,

    keyless_jwt: Option<String>,
}

impl Default for EmitJobRequest {
    fn default() -> Self {
        Self {
            rest_clients: Vec::new(),
            mode: EmitJobMode::MaxLoad {
                mempool_backlog: 3000,
            },
            transaction_mix_per_phase: vec![vec![(TransactionType::default(), 1)]],
            max_gas_per_txn: aptos_global_constants::MAX_GAS_AMOUNT,
            gas_price: aptos_global_constants::GAS_UNIT_PRICE,
            init_max_gas_per_txn: None,
            init_gas_price_multiplier: 2,
            mint_to_root: false,
            skip_funding_accounts: false,
            txn_expiration_time_secs: 60,
            init_expiration_multiplier: 3.0,
            init_retry_interval: Duration::from_secs(10),
            num_accounts_mode: NumAccountsMode::TransactionsPerAccount(20),
            expected_max_txns: MAX_TXNS,
            expected_gas_per_txn: None,
            expected_gas_per_transfer: EXPECTED_GAS_PER_TRANSFER,
            expected_gas_per_account_create: EXPECTED_GAS_PER_ACCOUNT_CREATE,
            prompt_before_spending: false,
            coordination_delay_between_instances: Duration::from_secs(0),
            latency_polling_interval: Duration::from_millis(300),
            tps_wait_after_expiration_secs: None,
            account_minter_seed: None,
            coins_per_account_override: None,
            account_type: AccountType::Local,
            keyless_ephem_secret_key: None,
            proof_file_path: None,
            epk_expiry_date_secs: None,
            keyless_jwt: None,
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

    pub fn max_gas_per_txn(mut self, max_gas_per_txn: u64) -> Self {
        self.max_gas_per_txn = max_gas_per_txn;
        self
    }

    pub fn init_expiration_multiplier(mut self, init_expiration_multiplier: f64) -> Self {
        self.init_expiration_multiplier = init_expiration_multiplier;
        self
    }

    pub fn init_max_gas_per_txn(mut self, init_max_gas_per_txn: u64) -> Self {
        self.init_max_gas_per_txn = Some(init_max_gas_per_txn);
        self
    }

    pub fn init_gas_price_multiplier(mut self, init_gas_price_multiplier: u64) -> Self {
        self.init_gas_price_multiplier = init_gas_price_multiplier;
        self
    }

    pub fn expected_max_txns(mut self, expected_max_txns: u64) -> Self {
        self.expected_max_txns = expected_max_txns;
        self
    }

    pub fn expected_gas_per_txn(mut self, expected_gas_per_txn: u64) -> Self {
        self.expected_gas_per_txn = Some(expected_gas_per_txn);
        self
    }

    pub fn expected_gas_per_transfer(mut self, expected_gas_per_transfer: u64) -> Self {
        self.expected_gas_per_transfer = expected_gas_per_transfer;
        self
    }

    pub fn expected_gas_per_account_create(mut self, expected_gas_per_account_create: u64) -> Self {
        self.expected_gas_per_account_create = expected_gas_per_account_create;
        self
    }

    pub fn prompt_before_spending(mut self) -> Self {
        self.prompt_before_spending = true;
        self
    }

    pub fn transaction_type(mut self, transaction_type: TransactionType) -> Self {
        self.transaction_mix_per_phase = vec![vec![(transaction_type, 1)]];
        self
    }

    pub fn transaction_mix(mut self, transaction_mix: Vec<(TransactionType, usize)>) -> Self {
        self.transaction_mix_per_phase = vec![transaction_mix];
        self
    }

    pub fn transaction_mix_per_phase(
        mut self,
        transaction_mix_per_phase: Vec<Vec<(TransactionType, usize)>>,
    ) -> Self {
        self.transaction_mix_per_phase = transaction_mix_per_phase;
        self
    }

    pub fn account_type(mut self, account_type: AccountType) -> Self {
        self.account_type = account_type;
        self
    }

    pub fn keyless_ephem_secret_key_from_seed(mut self, seed_string: &str) -> Self {
        self.keyless_ephem_secret_key = Some(parse_seed(seed_string));
        self
    }

    pub fn keyless_ephem_secret_key(mut self, keyless_ephem_secret_key: Ed25519PrivateKey) -> Self {
        self.keyless_ephem_secret_key = Some(keyless_ephem_secret_key.to_bytes());
        self
    }

    pub fn proof_file_path(mut self, proof_file_path: &str) -> Self {
        self.proof_file_path = Some(proof_file_path.to_owned());
        self
    }

    pub fn epk_expiry_date_secs(mut self, epk_expiry_date_secs: u64) -> Self {
        self.epk_expiry_date_secs = Some(epk_expiry_date_secs);
        self
    }

    pub fn keyless_jwt(mut self, keyless_jwt: &str) -> Self {
        self.keyless_jwt = Some(keyless_jwt.to_owned());
        self
    }

    pub fn get_num_phases(&self) -> usize {
        self.transaction_mix_per_phase.len()
    }

    pub fn mode(mut self, mode: EmitJobMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn txn_expiration_time_secs(mut self, txn_expiration_time_secs: u64) -> Self {
        self.txn_expiration_time_secs = txn_expiration_time_secs;
        self
    }

    pub fn num_accounts_mode(mut self, num_accounts: NumAccountsMode) -> Self {
        self.num_accounts_mode = num_accounts;
        self
    }

    pub fn coordination_delay_between_instances(
        mut self,
        coordination_delay_between_instances: Duration,
    ) -> Self {
        self.coordination_delay_between_instances = coordination_delay_between_instances;
        self
    }

    pub fn latency_polling_interval(mut self, latency_polling_interval: Duration) -> Self {
        self.latency_polling_interval = latency_polling_interval;
        self
    }

    pub fn account_minter_seed(mut self, seed_string: &str) -> Self {
        self.account_minter_seed = Some(parse_seed(seed_string));
        self
    }

    pub fn coins_per_account_override(mut self, coins: u64) -> Self {
        self.coins_per_account_override = Some(coins);
        self
    }

    pub fn set_mint_to_root(mut self) -> Self {
        self.mint_to_root = true;
        self
    }

    pub fn skip_funding_accounts(mut self) -> Self {
        self.skip_funding_accounts = true;
        self
    }

    pub fn get_init_max_gas_per_txn(&self) -> u64 {
        self.init_max_gas_per_txn.unwrap_or(self.max_gas_per_txn)
    }

    pub fn get_expected_gas_per_txn(&self) -> u64 {
        self.expected_gas_per_txn.unwrap_or(self.max_gas_per_txn)
    }

    pub fn get_expected_gas_per_transfer(&self) -> u64 {
        self.expected_gas_per_transfer
    }

    pub fn get_expected_gas_per_account_create(&self) -> u64 {
        self.expected_gas_per_account_create
    }

    pub fn get_init_gas_price(&self) -> u64 {
        self.gas_price * self.init_gas_price_multiplier
    }

    pub fn calculate_mode_params(&self) -> EmitModeParams {
        let clients_count = self.rest_clients.len();
        assert!(clients_count > 0, "No rest clients provided");

        match self.mode {
            EmitJobMode::MaxLoad { mempool_backlog } => {
                // The target mempool backlog is set to be 3x of the target TPS because of the on an average,
                // we can ~3 blocks in consensus queue. As long as we have 3x the target TPS as backlog,
                // it should be enough to produce the target TPS.
                let (transactions_per_account, num_accounts) = match self.num_accounts_mode {
                    NumAccountsMode::NumAccounts(num_accounts) => {
                        assert_eq!(
                            mempool_backlog % num_accounts,
                            0,
                            "mempool_backlog should be a multiple of num_accounts"
                        );
                        (mempool_backlog / num_accounts, num_accounts)
                    },
                    NumAccountsMode::TransactionsPerAccount(transactions_per_account) => (
                        transactions_per_account,
                        mempool_backlog / transactions_per_account,
                    ),
                };

                assert!(
                    transactions_per_account > 0,
                    "mempool_backlog smaller than num_accounts"
                );
                assert!(
                    num_accounts > 0,
                    "mempool_backlog smaller than transactions_per_account"
                );

                info!(
                    " Transaction emitter target mempool backlog is {}",
                    mempool_backlog
                );

                info!(
                    " Will use {} clients and {} total number of accounts",
                    clients_count, num_accounts
                );

                EmitModeParams {
                    wait_millis: 0,
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    num_accounts,
                    transactions_per_account,
                    max_submit_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
                    worker_offset_mode: WorkerOffsetMode::Jitter {
                        jitter_millis: 5000,
                    },
                    endpoints: clients_count,
                    check_account_sequence_only_once_fraction: 0.0,
                    check_account_sequence_sleep: self.latency_polling_interval,
                }
            },
            EmitJobMode::ConstTps { tps }
            | EmitJobMode::WaveTps {
                average_tps: tps, ..
            } => {
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

                let wait_seconds =
                    if let Some(wait_after_expiration) = self.tps_wait_after_expiration_secs {
                        self.txn_expiration_time_secs + wait_after_expiration
                    } else {
                        self.txn_expiration_time_secs * 2 + 5
                    };
                // In case we set a very low TPS, we need to still be able to spread out
                // transactions, at least to the seconds granularity, so we reduce transactions_per_account
                // if needed.
                let transactions_per_account = match self.num_accounts_mode {
                    NumAccountsMode::TransactionsPerAccount(transactions_per_account) => {
                        transactions_per_account
                    },
                    _ => 10,
                };
                let transactions_per_account = min(transactions_per_account, tps);
                assert!(
                    transactions_per_account > 0,
                    "TPS ({}) needs to be larger than 0",
                    tps,
                );
                let num_accounts = match self.num_accounts_mode {
                    NumAccountsMode::NumAccounts(num_accounts) => num_accounts,
                    NumAccountsMode::TransactionsPerAccount(_) => {
                        let total_txns = tps * wait_seconds as usize;
                        let num_accounts = total_txns / transactions_per_account;
                        assert!(num_accounts > 0, "Requested too small TPS: {}", tps);
                        num_accounts
                    },
                };

                info!(
                    " Transaction emitter targetting {} TPS, expecting {} TPS",
                    tps,
                    num_accounts * transactions_per_account / wait_seconds as usize
                );

                info!(
                    " Transaction emitter transactions_per_account batch is {}, with wait_seconds {}",
                    transactions_per_account, wait_seconds
                );

                // sample latency on 2% of requests, or at least once every 5s.
                let sample_latency_fraction =
                    1.0_f32.min(0.02_f32.max(wait_seconds as f32 / num_accounts as f32 / 5.0_f32));

                info!(
                    " Will use {} clients and {} accounts, sampling latency on {}",
                    clients_count, num_accounts, sample_latency_fraction
                );

                EmitModeParams {
                    wait_millis: wait_seconds * 1000,
                    txn_expiration_time_secs: self.txn_expiration_time_secs,
                    num_accounts,
                    transactions_per_account,
                    max_submit_batch_size: DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
                    worker_offset_mode: if let EmitJobMode::WaveTps {
                        wave_ratio,
                        num_waves,
                        ..
                    } = self.mode
                    {
                        WorkerOffsetMode::Wave {
                            wave_ratio: wave_ratio as f64,
                            num_waves: num_waves as f64,
                        }
                    } else {
                        WorkerOffsetMode::Spread
                    },
                    endpoints: clients_count,
                    check_account_sequence_only_once_fraction: 1.0 - sample_latency_fraction,
                    check_account_sequence_sleep: self.latency_polling_interval,
                }
            },
        }
    }
}

impl EmitModeParams {
    pub fn get_all_start_sleep_durations(&self, mut rng: ::rand::rngs::StdRng) -> Vec<Duration> {
        let index_range = 0..self.num_accounts;
        match self.worker_offset_mode {
            WorkerOffsetMode::NoOffset => index_range.map(|_i| 0).collect(),
            WorkerOffsetMode::Jitter { jitter_millis } => index_range
                .map(|_i| {
                    if jitter_millis > 0 {
                        rng.gen_range(0, jitter_millis)
                    } else {
                        0
                    }
                })
                .collect(),
            WorkerOffsetMode::Spread => index_range
                .map(|i| {
                    let start_offset_multiplier_millis =
                        self.wait_millis as f64 / (self.num_accounts) as f64;
                    (start_offset_multiplier_millis * i as f64) as u64
                })
                .collect(),
            WorkerOffsetMode::Wave {
                wave_ratio,
                num_waves,
            } => {
                // integral (1 - wave_ratio cos((2PI num_waves x)/wait_millis)) dx =
                // (x - (wave_ratio wait_millis sin( (num_waves 2PI x)/wait_millis ))  /  (num_waves 2PI))

                let time_scale = 2.0 * std::f64::consts::PI * num_waves;

                let integral = |time: f64| -> f64 {
                    time - (wave_ratio
                        * self.wait_millis as f64
                        * ((time_scale * time) / self.wait_millis as f64).sin())
                        / time_scale
                };

                let workers = self.num_accounts;
                let multiplier = workers as f64 / integral(self.wait_millis as f64);

                let mut result = Vec::new();
                for time in (0..self.wait_millis).step_by(10) {
                    let wanted = (multiplier * integral(time as f64)) as usize;
                    while wanted > result.len() {
                        result.push(time);
                    }
                }
                while workers > result.len() {
                    result.push(self.wait_millis);
                }
                result
            },
        }
        .into_iter()
        .map(Duration::from_millis)
        .collect()
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
    phase_starts: Vec<Instant>,
}

impl EmitJob {
    pub fn start_next_phase(&mut self) {
        let cur_phase = self.stats.start_next_phase();

        assert!(self.phase_starts.len() == cur_phase);
        self.phase_starts.push(Instant::now());
    }

    pub fn get_cur_phase(&self) -> usize {
        self.stats.get_cur_phase()
    }

    pub async fn stop_and_accumulate(self) -> Vec<TxnStats> {
        self.stop.store(true, Ordering::Relaxed);
        for worker in self.workers {
            let _accounts = worker
                .join_handle
                .await
                .expect("TxnEmitter worker thread failed");
        }

        self.stats.accumulate(&self.phase_starts)
    }

    pub fn peek_and_accumulate(&self) -> Vec<TxnStats> {
        self.stats.accumulate(&self.phase_starts)
    }

    pub async fn stop_job(self) -> Vec<TxnStats> {
        self.stop_and_accumulate().await
    }

    pub async fn periodic_stat(&self, duration: Duration, interval_secs: u64) {
        let deadline = Instant::now() + duration;
        let mut prev_stats: Option<Vec<TxnStats>> = None;
        let default_stats = TxnStats::default();
        let window = Duration::from_secs(max(interval_secs, 1));
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            tokio::time::sleep(window.min(left)).await;
            let cur_phase = self.stats.get_cur_phase();
            let stats = self.peek_and_accumulate();
            let delta = &stats[cur_phase]
                - prev_stats
                    .as_ref()
                    .map(|p| &p[cur_phase])
                    .unwrap_or(&default_stats);
            prev_stats = Some(stats);
            info!(
                "[{:?}s stat] phase {}: {}",
                window.as_secs(),
                cur_phase,
                delta.rate()
            );
        }
    }

    pub async fn periodic_stat_forward(self, duration: Duration, interval_secs: u64) -> Self {
        self.periodic_stat(duration, interval_secs).await;
        self
    }
}

#[derive(Clone, Debug)]
pub struct TxnEmitter {
    txn_factory: TransactionFactory,
    rng: StdRng,
    rest_cli: RestClient,
}

impl TxnEmitter {
    pub fn new(transaction_factory: TransactionFactory, rng: StdRng, rest_cli: RestClient) -> Self {
        Self {
            txn_factory: transaction_factory,
            rng,
            rest_cli,
        }
    }

    pub fn rng(&mut self) -> &mut StdRng {
        &mut self.rng
    }

    pub fn from_rng(&mut self) -> StdRng {
        StdRng::from_rng(self.rng()).unwrap()
    }

    pub async fn start_job(
        &mut self,
        root_account: Arc<LocalAccount>,
        req: EmitJobRequest,
        stats_tracking_phases: usize,
    ) -> Result<EmitJob> {
        ensure!(req.gas_price > 0, "gas_price is required to be non zero");

        let mode_params = req.calculate_mode_params();
        let num_accounts = mode_params.num_accounts;

        info!(
            "Will use total of {} endpoint clients and {} accounts",
            num_accounts, num_accounts
        );

        let txn_factory = self
            .txn_factory
            .clone()
            .with_transaction_expiration_time(mode_params.txn_expiration_time_secs)
            .with_gas_unit_price(req.gas_price)
            .with_max_gas_amount(req.max_gas_per_txn);

        let init_expiration_time =
            (mode_params.txn_expiration_time_secs as f64 * req.init_expiration_multiplier) as u64;
        let init_txn_factory = txn_factory
            .clone()
            .with_max_gas_amount(req.get_init_max_gas_per_txn())
            .with_gas_unit_price(req.get_init_gas_price())
            .with_transaction_expiration_time(init_expiration_time);
        let init_retries: usize =
            usize::try_from(init_expiration_time / req.init_retry_interval.as_secs()).unwrap();
        let account_generator = match req.account_type {
            AccountType::Local => create_private_key_account_generator(),
            AccountType::Keyless => {
                let ephem_sk = Ed25519PrivateKey::try_from(
                    req.keyless_ephem_secret_key
                        .expect("keyless_ephem_secret_key to not be None")
                        .as_ref(),
                )?;
                let keyless_config = self
                    .rest_cli
                    .get_resource(AccountAddress::ONE, "0x1::keyless_account::Configuration")
                    .await?
                    .into_inner();
                create_keyless_account_generator(
                    ephem_sk,
                    req.epk_expiry_date_secs
                        .expect("epk_expiry_date_secs to not be None"),
                    req.keyless_jwt
                        .as_deref()
                        .expect("keyless_jwt to not be None"),
                    req.proof_file_path.as_deref(),
                    keyless_config,
                )?
            },
        };

        let mut all_accounts = bulk_create_accounts(
            root_account.clone(),
            &RestApiReliableTransactionSubmitter::new(
                req.rest_clients.clone(),
                init_retries,
                req.init_retry_interval,
            ),
            &init_txn_factory,
            account_generator,
            (&req).into(),
            num_accounts,
            get_needed_balance_per_account_from_req(&req, num_accounts),
        )
        .await?;

        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(DynamicStatsTracking::new(stats_tracking_phases));
        let tokio_handle = Handle::current();

        let txn_executor = RestApiReliableTransactionSubmitter::new(
            req.rest_clients.clone(),
            init_retries,
            req.init_retry_interval,
        );
        let source_account_manager = SourceAccountManager {
            source_account: root_account.clone(),
            txn_executor: &txn_executor,
            txn_factory: init_txn_factory.clone(),
            mint_to_root: req.mint_to_root,
            prompt_before_spending: req.prompt_before_spending,
        };
        let (txn_generator_creator, _, _) = create_txn_generator_creator(
            req.transaction_mix_per_phase,
            source_account_manager,
            &mut all_accounts,
            vec![],
            &txn_executor,
            &txn_factory,
            &init_txn_factory,
            stats.get_cur_phase_obj(),
        )
        .await;

        if !req.coordination_delay_between_instances.is_zero() {
            info!(
                "Sleeping after minting/txn generator initialization for {}s",
                req.coordination_delay_between_instances.as_secs()
            );
            tokio::time::sleep(req.coordination_delay_between_instances).await;
        }

        let check_account_sequence_only_once_for = (0..num_accounts)
            .choose_multiple(
                &mut self.from_rng(),
                (mode_params.check_account_sequence_only_once_fraction * num_accounts as f32)
                    as usize,
            )
            .into_iter()
            .collect::<HashSet<_>>();

        info!(
            "Checking account sequence and counting latency for {} out of {} total_workers",
            num_accounts - check_account_sequence_only_once_for.len(),
            num_accounts
        );

        let all_start_sleep_durations = mode_params.get_all_start_sleep_durations(self.from_rng());

        // Creating workers is slow with many workers (TODO check why)
        // so we create them all first, before starting them - so they start at the right time for
        // traffic pattern to be correct.
        info!("Tx emitter creating workers");
        let mut submission_workers = Vec::with_capacity(num_accounts);
        let all_clients = Arc::new(req.rest_clients.clone());
        for index in 0..num_accounts {
            let main_client_index = index % all_clients.len();

            let accounts = all_accounts.split_off(all_accounts.len() - 1);
            let stop = stop.clone();
            let stats = Arc::clone(&stats);
            let txn_generator = txn_generator_creator.create_transaction_generator();
            let worker_index = submission_workers.len();

            let worker = SubmissionWorker::new(
                accounts,
                all_clients.clone(),
                main_client_index,
                stop,
                mode_params.clone(),
                stats,
                txn_generator,
                all_start_sleep_durations[worker_index],
                check_account_sequence_only_once_for.contains(&worker_index),
                self.from_rng(),
            );
            submission_workers.push(worker);
        }

        info!("Tx emitter workers created");
        let phase_start = Instant::now();
        let workers = submission_workers
            .into_iter()
            .map(|worker| Worker {
                join_handle: tokio_handle.spawn(worker.run(phase_start).boxed()),
            })
            .collect();
        info!("Tx emitter workers started");

        Ok(EmitJob {
            workers,
            stop,
            stats,
            phase_starts: vec![phase_start],
        })
    }

    async fn emit_txn_for_impl(
        mut self,
        source_account: Arc<LocalAccount>,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        print_stats_interval: Option<u64>,
    ) -> Result<TxnStats> {
        let phases = emit_job_request.transaction_mix_per_phase.len();

        let mut job = self
            .start_job(source_account, emit_job_request, phases)
            .await?;
        info!(
            "Starting emitting txns for {} secs in {} phases",
            duration.as_secs(),
            phases
        );

        let per_phase_duration = duration.checked_div(phases as u32).unwrap();
        for phase in 0..phases {
            if phase > 0 {
                info!("Starting next phase");
                job.start_next_phase();
            }
            if let Some(interval_secs) = print_stats_interval {
                job.periodic_stat(per_phase_duration, interval_secs).await;
            } else {
                time::sleep(per_phase_duration).await;
            }
        }
        info!("Ran for {} secs, stopping job...", duration.as_secs());
        let stats = job.stop_job().await;
        info!("Stopped job");
        Ok(stats.into_iter().next().unwrap())
    }

    pub async fn emit_txn_for(
        self,
        source_account: Arc<LocalAccount>,
        emit_job_request: EmitJobRequest,
        duration: Duration,
    ) -> Result<TxnStats> {
        self.emit_txn_for_impl(source_account, emit_job_request, duration, None)
            .await
    }

    pub async fn emit_txn_for_with_stats(
        self,
        source_account: Arc<LocalAccount>,
        emit_job_request: EmitJobRequest,
        duration: Duration,
        interval_secs: u64,
    ) -> Result<TxnStats> {
        self.emit_txn_for_impl(
            source_account,
            emit_job_request,
            duration,
            Some(interval_secs),
        )
        .await
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

#[allow(dead_code)]
fn pick_client(clients: &[RestClient]) -> &RestClient {
    clients.choose(&mut rand::thread_rng()).unwrap()
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
    account_seqs: &HashMap<AccountAddress, (u64, u64)>,
    txn_expiration_ts_secs: u64,
    sleep_between_cycles: Duration,
) -> (HashMap<AccountAddress, u64>, u128) {
    let mut pending_addresses: HashSet<_> = account_seqs.keys().copied().collect();
    let mut latest_fetched_counts = HashMap::new();

    let mut sum_of_completion_timestamps_millis = 0u128;
    loop {
        match query_sequence_numbers(client, pending_addresses.iter()).await {
            Ok((sequence_numbers, ledger_timestamp_secs)) => {
                let millis_elapsed = start_time.elapsed().as_millis();
                for (address, sequence_number) in sequence_numbers {
                    let (start_seq_num, end_seq_num) = account_seqs.get(&address).unwrap();

                    let prev_sequence_number = latest_fetched_counts
                        .insert(address, sequence_number)
                        .unwrap_or(*start_seq_num);
                    // fetched sequence number that is older than one we already fetched.
                    // client connection probably moved to a different server.
                    if prev_sequence_number <= sequence_number {
                        sum_of_completion_timestamps_millis +=
                            millis_elapsed * (sequence_number - prev_sequence_number) as u128;

                        if *end_seq_num == sequence_number {
                            pending_addresses.remove(&address);
                        }
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
                            pending_addresses,
                        )
                    );
                    break;
                }
            },
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
            },
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

    (latest_fetched_counts, sum_of_completion_timestamps_millis)
}

pub async fn query_sequence_number(client: &RestClient, address: AccountAddress) -> Result<u64> {
    Ok(query_sequence_numbers(client, [address].iter()).await?.0[0].1)
}

// Return a pair of (list of sequence numbers, ledger timestamp)
pub async fn query_sequence_numbers<'a, I>(
    client: &RestClient,
    addresses: I,
) -> Result<(Vec<(AccountAddress, u64)>, u64)>
where
    I: Iterator<Item = &'a AccountAddress>,
{
    let futures = addresses.map(|address| {
        FETCH_ACCOUNT_RETRY_POLICY.retry(move || get_account_address_and_seq_num(client, *address))
    });

    let (seq_nums, timestamps): (Vec<_>, Vec<_>) = try_join_all(futures)
        .await
        .map_err(|e| format_err!("Get accounts failed: {:?}", e))?
        .into_iter()
        .unzip();

    // return min for the timestamp, to make sure
    // all sequence numbers were <= to return values at that timestamp
    Ok((seq_nums, timestamps.into_iter().min().unwrap()))
}

async fn get_account_address_and_seq_num(
    client: &RestClient,
    address: AccountAddress,
) -> Result<((AccountAddress, u64), u64)> {
    get_account_seq_num(client, address)
        .await
        .map(|(seq_num, ts)| ((address, seq_num), ts))
}

pub async fn get_account_seq_num(
    client: &RestClient,
    address: AccountAddress,
) -> Result<(u64, u64)> {
    let result = client.get_account_bcs(address).await;
    match &result {
        Ok(resp) => Ok((
            resp.inner().sequence_number(),
            Duration::from_micros(resp.state().timestamp_usecs).as_secs(),
        )),
        Err(e) => {
            // if account is not present, that is equivalent to sequence_number = 0
            if let RestError::Api(api_error) = e {
                if let AptosErrorCode::AccountNotFound = api_error.error.error_code {
                    return Ok((
                        0,
                        Duration::from_micros(api_error.state.as_ref().unwrap().timestamp_usecs)
                            .as_secs(),
                    ));
                }
            }
            result?;
            unreachable!()
        },
    }
}

pub async fn load_specific_account(
    account_key: AccountKey,
    is_root: bool,
    client: &RestClient,
) -> Result<LocalAccount> {
    let address = if is_root {
        aptos_test_root_address()
    } else {
        account_key.authentication_key().account_address()
    };

    let sequence_number = query_sequence_number(client, address).await.map_err(|e| {
        format_err!(
            "query_sequence_number on {:?} for account {} failed: {:?}",
            client,
            address,
            e
        )
    })?;
    Ok(LocalAccount::new(address, account_key, sequence_number))
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

pub fn parse_seed(seed_string: &str) -> [u8; 32] {
    // Remove the brackets and spaces
    let cleaned_string = seed_string
        .trim_start_matches('[')
        .trim_end_matches(']')
        .replace(' ', "");

    // Parse the cleaned string into a vector
    let parsed_vector: Result<Vec<u8>, _> = cleaned_string.split(',').map(u8::from_str).collect();

    <[u8; 32]>::try_from(parsed_vector.expect("failed to parse seed"))
        .expect("failed to convert to array")
}

pub fn get_needed_balance_per_account(
    num_workload_transactions: u64,
    gas_per_workload_transaction: u64,
    octas_per_workload_transaction: u64,
    num_accounts: usize,
    gas_price: u64,
    max_gas_per_txn: u64,
) -> u64 {
    // round up:
    let txnx_per_account = num_workload_transactions.div_ceil(num_accounts as u64);
    let coins_per_account = txnx_per_account
        .checked_mul(octas_per_workload_transaction + gas_per_workload_transaction * gas_price)
        .unwrap()
        .checked_add(max_gas_per_txn * gas_price)
        .unwrap();

    info!(
        "Needed {} balance for each account because of expecting {} txns per account with {} gas and {} octas, with leaving {} gas for max_txn_gas, all at {} gas price",
        coins_per_account,
        txnx_per_account,
        gas_per_workload_transaction,
        octas_per_workload_transaction,
        max_gas_per_txn,
        gas_price,
    );
    coins_per_account
}

pub fn get_needed_balance_per_account_from_req(req: &EmitJobRequest, num_accounts: usize) -> u64 {
    if let Some(val) = req.coins_per_account_override {
        info!(
            "Needed {} balance for each account because of override",
            val
        );
        val
    } else {
        get_needed_balance_per_account(
            req.expected_max_txns,
            req.get_expected_gas_per_txn(),
            SEND_AMOUNT,
            num_accounts,
            req.gas_price,
            req.max_gas_per_txn,
        )
    }
}
