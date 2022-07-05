// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod account_minter;
pub mod stats;
pub mod submission_worker;

use ::aptos_logger::*;
use again::RetryPolicy;
use anyhow::{format_err, Result};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use futures::future::{try_join_all, FutureExt};
use itertools::zip;
use once_cell::sync::Lazy;
use rand_core::SeedableRng;
use std::{
    cmp::{max, min},
    collections::HashSet,
    num::NonZeroU64,
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
        account_generator::AccountGenerator,
        nft_mint::{initialize_nft_collection, NFTMint},
        p2p_transaction_generator::P2PTransactionGenerator,
        TransactionGenerator,
    },
};
use aptos_sdk::transaction_builder::aptos_stdlib;
use rand::rngs::StdRng;
use stats::{StatsAccumulator, TxnStats};

/// Max transactions per account in mempool
const MAX_TXN_BATCH_SIZE: usize = 100;
const MAX_TXNS: u64 = 1_000_000;
const SEND_AMOUNT: u64 = 1;
const TXN_EXPIRATION_SECONDS: u64 = 180;
const TXN_MAX_WAIT: Duration = Duration::from_secs(TXN_EXPIRATION_SECONDS as u64 + 30);

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
pub struct EmitThreadParams {
    pub wait_millis: u64,
    pub wait_committed: bool,
    pub txn_expiration_time_secs: u64,
    pub check_stats_at_end: bool,
}

impl Default for EmitThreadParams {
    fn default() -> Self {
        Self {
            wait_millis: 0,
            wait_committed: true,
            txn_expiration_time_secs: 30,
            check_stats_at_end: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EmitJobRequest {
    rest_clients: Vec<RestClient>,
    accounts_per_client: usize,
    workers_per_endpoint: Option<usize>,
    thread_params: EmitThreadParams,
    gas_price: u64,
    invalid_transaction_ratio: usize,
    vasp: bool,
    transaction_type: TransactionType,
}

impl Default for EmitJobRequest {
    fn default() -> Self {
        Self {
            rest_clients: Vec::new(),
            accounts_per_client: 15,
            workers_per_endpoint: None,
            thread_params: EmitThreadParams::default(),
            gas_price: 0,
            invalid_transaction_ratio: 0,
            vasp: false,
            transaction_type: TransactionType::P2P,
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

    pub fn accounts_per_client(mut self, accounts_per_client: usize) -> Self {
        self.accounts_per_client = accounts_per_client;
        self
    }

    pub fn workers_per_endpoint(mut self, workers_per_endpoint: usize) -> Self {
        self.workers_per_endpoint = Some(workers_per_endpoint);
        self
    }

    pub fn thread_params(mut self, thread_params: EmitThreadParams) -> Self {
        self.thread_params = thread_params;
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

    pub fn fixed_tps(self, target_tps: NonZeroU64) -> Self {
        let clients_count = self.rest_clients.len() as u64;
        let num_workers = target_tps.get() / clients_count + 1;
        let wait_time = clients_count * num_workers * 1000 / target_tps.get();

        self.workers_per_endpoint(num_workers as usize)
            .thread_params(EmitThreadParams {
                wait_millis: wait_time,
                wait_committed: true,
                txn_expiration_time_secs: 30,
                check_stats_at_end: true,
            })
            .accounts_per_client(1)
    }

    pub fn vasp(mut self) -> Self {
        self.vasp = true;
        self
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
        let workers_per_endpoint = match req.workers_per_endpoint {
            Some(x) => x,
            None => {
                let target_threads = 300;
                // Trying to create somewhere between target_threads/2..target_threads threads
                // We want to have equal numbers of threads for each endpoint, so that they are equally loaded
                // Otherwise things like flamegrap/perf going to show different numbers depending on which endpoint is chosen
                // Also limiting number of threads as max 10 per endpoint for use cases with very small number of nodes or use --peers
                min(10, max(1, target_threads / req.rest_clients.len()))
            }
        };
        let num_clients = req.rest_clients.len() * workers_per_endpoint;
        info!(
            "Will use {} workers per endpoint for a total of {} endpoint clients",
            workers_per_endpoint, num_clients
        );
        let num_accounts = req.accounts_per_client * num_clients;
        info!(
            "Will create {} accounts_per_client for a total of {} accounts",
            req.accounts_per_client, num_accounts
        );
        let mut account_minter = AccountMinter::new(
            self.root_account,
            self.txn_factory.clone(),
            self.rng.clone(),
        );
        let mut new_accounts = account_minter.mint_accounts(&req, num_accounts).await?;
        self.accounts.append(&mut new_accounts);
        let all_accounts = self.accounts.split_off(self.accounts.len() - num_accounts);
        let mut workers = vec![];
        let all_addresses: Vec<_> = all_accounts.iter().map(|d| d.address()).collect();
        let all_addresses = Arc::new(all_addresses);
        let mut all_accounts = all_accounts.into_iter();
        let stop = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(StatsAccumulator::default());
        let tokio_handle = Handle::current();
        let mut creator_account = LocalAccount::generate(&mut self.rng);
        let collection_name = "collection name".to_owned().into_bytes();
        let token_name = "token name".to_owned().into_bytes();
        if let TransactionType::NftMint = req.transaction_type {
            initialize_nft_collection(
                req.rest_clients[0].clone(),
                self.root_account,
                &mut creator_account,
                &self.txn_factory,
                &collection_name,
                &token_name,
            )
            .await;
        };
        let nft_creator_account = Arc::new(creator_account);
        for client in req.rest_clients {
            for _ in 0..workers_per_endpoint {
                let accounts = (&mut all_accounts).take(req.accounts_per_client).collect();
                let all_addresses = all_addresses.clone();
                let stop = stop.clone();
                let params = req.thread_params.clone();
                let stats = Arc::clone(&stats);
                let txn_generator: Box<dyn TransactionGenerator> = match req.transaction_type {
                    TransactionType::P2P => Box::new(P2PTransactionGenerator::new(
                        self.from_rng().clone(),
                        SEND_AMOUNT,
                        self.txn_factory.clone(),
                    )),
                    TransactionType::AccountGeneration => Box::new(AccountGenerator::new(
                        self.from_rng().clone(),
                        self.txn_factory.clone(),
                    )),
                    TransactionType::NftMint => {
                        let nft_mint = NFTMint::new(
                            self.txn_factory.clone(),
                            nft_creator_account.clone(),
                            collection_name.clone(),
                            token_name.clone(),
                        )
                        .await;
                        Box::new(nft_mint)
                    }
                };
                let worker = SubmissionWorker::new(
                    accounts,
                    client.clone(),
                    all_addresses,
                    stop,
                    params,
                    stats,
                    txn_generator,
                    req.invalid_transaction_ratio,
                    self.from_rng(),
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

    pub async fn emit_txn_for(
        &mut self,
        duration: Duration,
        emit_job_request: EmitJobRequest,
    ) -> Result<TxnStats> {
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
        duration: Duration,
        emit_job_request: EmitJobRequest,
        interval_secs: u64,
    ) -> Result<TxnStats> {
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
        client
            .submit(&gen_transfer_txn_request(
                sender,
                receiver,
                num_coins,
                &self.txn_factory,
                1,
            ))
            .await?;
        let deadline = Instant::now() + TXN_MAX_WAIT;
        Ok(deadline)
    }
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
    client: &RestClient,
    accounts: &mut [LocalAccount],
    wait_timeout: Duration,
) -> Result<(), HashSet<AccountAddress>> {
    let deadline = Instant::now() + wait_timeout;
    let addresses: Vec<_> = accounts.iter().map(|d| d.address()).collect();
    let mut uncommitted = addresses.clone().into_iter().collect::<HashSet<_>>();

    while Instant::now() <= deadline {
        match query_sequence_numbers(client, &addresses).await {
            Ok(sequence_numbers) => {
                for (account, sequence_number) in zip(accounts.iter(), &sequence_numbers) {
                    if account.sequence_number() == *sequence_number {
                        uncommitted.remove(&account.address());
                    }
                }

                if uncommitted.is_empty() {
                    return Ok(());
                }
            }
            Err(e) => {
                info!(
                    "Failed to query ledger info on accounts {:?} for instance {:?} : {:?}",
                    addresses, client, e
                );
            }
        }

        time::sleep(Duration::from_millis(250)).await;
    }

    Err(uncommitted)
}

pub async fn query_sequence_numbers(
    client: &RestClient,
    addresses: &[AccountAddress],
) -> Result<Vec<u64>> {
    Ok(try_join_all(
        addresses
            .iter()
            .map(|address| RETRY_POLICY.retry(move || client.get_account(*address))),
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
            .payload(aptos_stdlib::encode_test_coin_transfer(
                *receiver, num_coins,
            ))
            .gas_unit_price(gas_price),
    )
}
