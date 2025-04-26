// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    indexer::continuously_update_indexer_delay,
    metrics::{spawn_async_tracking, Tracking},
    workloads::SignedTransactionBuilder,
};
use anyhow::{bail, Result};
use aptos_config::config::DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE;
use aptos_logger::{error, info, sample, sample::SampleRate, warn};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    rest_client::{
        aptos_api_types::{AptosError, AptosErrorCode, TransactionOnChainData},
        error::{AptosErrorResponse, RestError},
        Client,
    },
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::LocalAccount,
};
use aptos_transaction_emitter_lib::{
    emitter::{
        account_minter::{bulk_create_accounts, prompt_yes, BulkAccountCreationConfig},
        get_account_seq_num, get_needed_balance_per_account,
        local_account_generator::{LocalAccountGenerator, PrivateKeyAccountGenerator},
        parse_seed,
        transaction_executor::RestApiReliableTransactionSubmitter,
        EXPECTED_GAS_PER_ACCOUNT_CREATE, EXPECTED_GAS_PER_TRANSFER,
    },
    Cluster, ClusterArgs,
};
use aptos_transaction_generator_lib::ReliableTransactionSubmitter;
use clap::Parser;
use futures::{future::join_all, StreamExt};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicI64, AtomicUsize},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Parser)]
pub struct TransactionFactoryArgs {
    #[clap(long)]
    pub octas_per_workload_transaction: u64,

    #[clap(long, default_value = "100")]
    gas_price: u64,

    #[clap(long)]
    init_gas_price: Option<u64>,

    #[clap(long, default_value = "10000")]
    init_max_gas_per_txn: u64,

    #[clap(long, default_value = "60")]
    expiration_time_s: u64,
}

impl TransactionFactoryArgs {
    pub fn get_workload_max_gas_amount(&self) -> u64 {
        self.octas_per_workload_transaction / self.gas_price + 1
    }

    pub fn with_init_params(&self, factory: TransactionFactory) -> TransactionFactory {
        factory
            .with_gas_unit_price(self.init_gas_price.unwrap_or(self.gas_price))
            .with_transaction_expiration_time(self.expiration_time_s)
            .with_max_gas_amount(self.init_max_gas_per_txn)
    }

    pub fn with_params(&self, factory: TransactionFactory) -> TransactionFactory {
        factory
            .with_gas_unit_price(self.gas_price)
            .with_transaction_expiration_time(self.expiration_time_s)
            .with_max_gas_amount(self.get_workload_max_gas_amount())
    }
}

#[derive(Parser, Debug)]
pub struct AccountsArgs {
    /// Number of accounts to create
    #[clap(long)]
    pub num_worker_accounts: usize,

    /// Optional seed for accounts used. If no seed is provided, a random seed is used and printed.
    #[clap(long)]
    pub accounts_seed: Option<String>,
}

#[derive(Parser, Debug)]
pub struct SubmitArgs {
    #[clap(flatten)]
    pub cluster_args: ClusterArgs,

    #[clap(flatten)]
    pub transaction_factory_args: TransactionFactoryArgs,

    #[clap(flatten)]
    pub accounts_args: AccountsArgs,

    #[clap(long)]
    skip_funding_accounts: bool,

    #[clap(long, default_value = "5")]
    batch_size: usize,
    #[clap(long, default_value = "0.3")]
    poll_interval_s: f32,

    #[clap(long)]
    output_file: Option<String>,
}

#[derive(Parser, Debug)]
pub struct CreateSampleAddresses {
    /// Number of accounts to create
    #[clap(long)]
    num_addresses: usize,

    #[clap(long)]
    output_file: String,
}

#[derive(Parser, Debug)]
pub struct SanitizeAddresses {
    #[clap(long)]
    pub destinations_file: String,

    #[clap(long)]
    pub output_file: String,
}

pub async fn execute_submit<T: Clone, B: SignedTransactionBuilder<T>>(
    work: Vec<T>,
    args: SubmitArgs,
    builder: B,
    cluster: Cluster,
    coin_source_account: LocalAccount,
    detailed_progress: bool,
) -> Result<()> {
    let clients = cluster
        .all_instances()
        .map(|i| i.rest_client())
        .collect::<Vec<_>>();
    let txn_factory = TransactionFactory::new(cluster.chain_id);

    let needed_balance_per_account = get_needed_balance_per_account(
        work.len() as u64,
        0,
        args.transaction_factory_args.octas_per_workload_transaction,
        args.accounts_args.num_worker_accounts,
        args.transaction_factory_args.gas_price,
        args.transaction_factory_args.get_workload_max_gas_amount(),
    );

    let worker_accounts = create_worker_accounts(
        clients.clone(),
        coin_source_account,
        args.transaction_factory_args
            .with_init_params(txn_factory.clone()),
        args.accounts_args.num_worker_accounts,
        args.accounts_args.accounts_seed.as_deref(),
        args.skip_funding_accounts,
        cluster.coin_source_is_root,
        needed_balance_per_account,
    )
    .await?;

    if !prompt_yes("About to submit transactions. Continue?") {
        bail!("User aborted")
    }

    let output = execute_txn_list(
        worker_accounts,
        clients,
        work,
        args.batch_size,
        args.batch_size,
        Duration::from_secs_f32(args.poll_interval_s),
        args.transaction_factory_args.with_params(txn_factory),
        builder,
        detailed_progress,
    )
    .await?;

    if let Some(output_file) = args.output_file {
        std::fs::write(output_file, output.join("\n"))?;
    } else {
        for txn in output {
            println!("{}", txn);
        }
    }

    Ok(())
}

pub async fn execute_return_worker_funds(
    transaction_factory_args: TransactionFactoryArgs,
    accounts_args: AccountsArgs,
    cluster: Cluster,
    coin_source_account: &LocalAccount,
) -> Result<()> {
    let return_funds_retries = 5;
    let return_funds_retry_interval = Duration::from_secs(3);

    let clients = cluster
        .all_instances()
        .map(|i| i.rest_client())
        .collect::<Vec<_>>();

    let txn_factory =
        transaction_factory_args.with_params(TransactionFactory::new(cluster.chain_id));

    let txn_executor = RestApiReliableTransactionSubmitter::new(
        clients,
        return_funds_retries,
        return_funds_retry_interval,
    );

    let accounts = PrivateKeyAccountGenerator
        .gen_local_accounts(
            &txn_executor,
            accounts_args.num_worker_accounts,
            &mut StdRng::from_seed(parse_seed(&accounts_args.accounts_seed.unwrap())),
        )
        .await?;

    let txn_executor_ref = &txn_executor;
    let counter = txn_executor_ref.create_counter_state();
    let counter_ref = &counter;
    let txn_factory_ref = &txn_factory;
    let _ = futures::stream::iter(accounts.iter().map(|account| async move {
        while let Ok(balance) = txn_executor_ref
            .get_account_balance(account.address())
            .await
        {
            eprintln!("account: {} balance: {}", account.address(), balance);
            if balance > txn_factory_ref.get_max_gas_amount() * txn_factory_ref.get_gas_unit_price()
            {
                let txn = account.sign_with_transaction_builder(txn_factory_ref.payload(
                    aptos_stdlib::aptos_coin_transfer(
                        coin_source_account.address(),
                        balance
                            - txn_factory_ref.get_max_gas_amount()
                                * txn_factory_ref.get_gas_unit_price(),
                    ),
                ));
                if txn_executor_ref
                    .execute_transactions_with_counter(&[txn], counter_ref)
                    .await
                    .is_ok()
                {
                    break;
                }
            } else {
                break;
            }
        }
        Ok::<(), ()>(())
    }))
    .buffered(100)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    Ok(())
}

async fn create_worker_accounts(
    clients: Vec<Client>,
    coin_source_account: LocalAccount,
    init_txn_factory: TransactionFactory,
    num_accounts: usize,
    accounts_seed: Option<&str>,
    skip_funding_accounts: bool,
    coin_source_is_root: bool,
    needed_balance_per_account: u64,
) -> Result<Vec<LocalAccount>> {
    let account_funding_retries = 5;
    let account_funding_retry_interval = Duration::from_secs(3);

    bulk_create_accounts(
        Arc::new(coin_source_account),
        &RestApiReliableTransactionSubmitter::new(
            clients,
            account_funding_retries,
            account_funding_retry_interval,
        ),
        &init_txn_factory,
        Box::new(PrivateKeyAccountGenerator),
        BulkAccountCreationConfig::new(
            DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE,
            skip_funding_accounts,
            accounts_seed,
            coin_source_is_root,
            true,
            false,
            EXPECTED_GAS_PER_TRANSFER,
            EXPECTED_GAS_PER_ACCOUNT_CREATE,
        ),
        num_accounts,
        needed_balance_per_account,
    )
    .await
}

struct AccountWork<T> {
    account: LocalAccount,
    work: Vec<T>,
    initial_seq_num: u64,
}

impl<T> AccountWork<T> {
    fn new(account: LocalAccount, work: Vec<T>) -> Self {
        let initial_seq_num = account.sequence_number();
        Self {
            account,
            work,
            initial_seq_num,
        }
    }
}

pub async fn execute_txn_list<T: Clone, B: SignedTransactionBuilder<T>>(
    accounts: Vec<LocalAccount>,
    clients: Vec<Client>,
    work: Vec<T>,
    single_request_api_batch_size: usize,
    parallel_requests_outstanding: usize,
    poll_interval: Duration,
    txn_factory: TransactionFactory,
    builder: B,
    detailed_progress: bool,
) -> Result<Vec<String>> {
    let mut work_chunks = (0..accounts.len()).map(|_| vec![]).collect::<Vec<_>>();
    for (i, work) in work.into_iter().enumerate() {
        work_chunks[i % accounts.len()].push(work);
    }
    assert_eq!(work_chunks.len(), accounts.len());

    let accounts_with_work = work_chunks
        .into_iter()
        .zip(accounts.into_iter())
        .map(|(work, account)| AccountWork::new(account, work))
        .collect::<Vec<_>>();
    let txn_factory = &txn_factory;
    let builder = &builder;
    let clients = &clients;

    let indexer_delay = Arc::new(AtomicI64::new(0));

    let tracking = Arc::new(Tracking::new(indexer_delay.clone(), detailed_progress));
    let tracking_ref = tracking.as_ref();

    let tracking_done = spawn_async_tracking(tracking.clone(), Duration::from_secs(2));

    let _task = tokio::spawn(continuously_update_indexer_delay(
        txn_factory.get_chain_id(),
        indexer_delay.clone(),
    ));
    let indexer_delay_ref = &indexer_delay;

    let start_time = Instant::now();

    join_all(
        accounts_with_work
            .iter()
            .map(|account_with_work| async move {
                submit_work_txns(
                    &account_with_work.account,
                    account_with_work.initial_seq_num,
                    &account_with_work.work,
                    single_request_api_batch_size,
                    parallel_requests_outstanding,
                    builder,
                    txn_factory,
                    clients,
                    poll_interval,
                    tracking_ref,
                    indexer_delay_ref,
                    &BackoffConfig::default(),
                )
                .await;
            }),
    )
    .await;

    let elapsed = start_time.elapsed().as_secs_f64();
    tracking_done.store(true, std::sync::atomic::Ordering::Relaxed);
    tokio::time::sleep(Duration::from_secs(1)).await;

    warn!("Done executing work, fetching outputs");
    tracking.print_stats(elapsed);

    let progress = Arc::new(AtomicUsize::new(0));
    let done_tracking = spawn_async_tracking(progress.clone(), Duration::from_secs(10));
    let progress_ref = progress.as_ref();
    let out = futures::stream::iter(accounts_with_work.into_iter().map(
        |account_with_work| async move {
            fetch_work_txn_output(
                &account_with_work.account,
                account_with_work.initial_seq_num,
                &account_with_work.work,
                clients,
                progress_ref,
            )
            .await
        },
    ))
    .buffered(400)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    done_tracking.store(true, std::sync::atomic::Ordering::Relaxed);
    tokio::time::sleep(Duration::from_secs(1)).await;

    for (status, infos) in group_pairs(out.iter().map(|txn| {
        (
            txn.1
                .as_ref()
                .map_or("missing".to_string(), |t| format!("{:?}", t.info.status())),
            txn,
        )
    }))
    .into_iter()
    {
        let gas_used = infos
            .iter()
            .map(|txn| txn.1.as_ref().map_or(0, |t| t.info.gas_used()))
            .collect::<Vec<_>>();
        info!(
            "{:?}: {} txns, gas used: min: {:?}, max: {:?}",
            status,
            infos.len(),
            gas_used.iter().min().unwrap(),
            gas_used.iter().max().unwrap(),
        );
    }

    let result = out
        .iter()
        .map(|(data, txn)| builder.success_output(data, txn))
        .collect::<Vec<_>>();

    Ok(result)
}

fn group_pairs<A, B, I>(v: I) -> BTreeMap<A, Vec<B>>
where
    A: Ord,
    I: IntoIterator<Item = (A, B)>,
{
    let mut result = BTreeMap::<A, Vec<B>>::new();
    for (a, b) in v {
        result.entry(a).or_default().push(b);
    }
    result
}

fn start_sleep_duration() -> Duration {
    Duration::from_secs_f64(rand::thread_rng().gen_range(0.0, 5.0))
}

struct SingleBackoffConfig {
    lower_threshold: f64,
    threshold_gap: f64,
}

impl SingleBackoffConfig {
    fn should_backoff(&self, delay: f64) -> bool {
        if delay > self.lower_threshold {
            // the bigger the delay, the more likely we should wait
            // if delay is above lower_threshold + threshold_gap, we completely pause the submission
            if rand::thread_rng().gen_bool(
                ((delay - self.lower_threshold) / self.threshold_gap)
                    .sqrt()
                    .min(1.0),
            ) {
                return true;
            }
        }
        false
    }
}

struct BackoffConfig {
    indexer_backoff: SingleBackoffConfig,
    blockchain_backoff: SingleBackoffConfig,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            indexer_backoff: SingleBackoffConfig {
                lower_threshold: 2.0,
                threshold_gap: 4.0,
            },
            blockchain_backoff: SingleBackoffConfig {
                lower_threshold: 1.2,
                threshold_gap: 1.0,
            },
        }
    }
}

async fn submit_work_txns<T, B: SignedTransactionBuilder<T>>(
    account: &LocalAccount,
    initial_seq_num: u64,
    work: &[T],
    single_request_api_batch_size: usize,
    parallel_requests_outstanding: usize,
    builder: &B,
    txn_factory: &TransactionFactory,
    clients: &[Client],
    poll_interval: Duration,
    tracking: &Tracking,
    indexer_delay_ref: &AtomicI64,
    backoff_config: &BackoffConfig,
) {
    tokio::time::sleep(start_sleep_duration()).await;

    let mut consecutive_rollback = 0;

    let mut indexer_backoffs = 0;
    let mut blockchain_backoffs = 0;

    'outer: loop {
        let indexer_delay = indexer_delay_ref.load(std::sync::atomic::Ordering::Relaxed) as f64;
        if backoff_config.indexer_backoff.should_backoff(indexer_delay) {
            tokio::time::sleep(Duration::from_secs(1)).await;
            indexer_backoffs += 1;
            continue;
        }

        let last_blockchain_latency = tracking.get_last_latency();
        if backoff_config
            .blockchain_backoff
            .should_backoff(last_blockchain_latency)
        {
            tokio::time::sleep(Duration::from_secs(1)).await;
            blockchain_backoffs += 1;
            continue;
        }

        let start_seq_num = account.sequence_number();
        let chunk_start = (start_seq_num - initial_seq_num) as usize;
        if chunk_start >= work.len() {
            break;
        }
        let chunk =
            &work[chunk_start..(work.len().min(chunk_start + parallel_requests_outstanding))];

        let txns = chunk
            .iter()
            .map(|data| builder.build(data, account, txn_factory))
            .collect::<Vec<_>>();

        let client = pick_client(clients);

        let before_submitted_instant = Instant::now();

        let min_failed = join_all(txns.chunks(single_request_api_batch_size).map(
            |batch| async move {
                match client.submit_batch_bcs(batch).await {
                    Err(e) => {
                        warn!("Error submitting batch: {:?}", e);
                        Some(batch.first().unwrap().sequence_number())
                    },
                    Ok(r) => {
                        let result = r.into_inner();
                        if !result.transaction_failures.is_empty() {
                            warn!("Failed submission: {:?}", result.transaction_failures);
                            let first_failure = result
                                .transaction_failures
                                .iter()
                                .map(|tf| tf.transaction_index)
                                .min()
                                .unwrap();
                            Some(batch[first_failure].sequence_number())
                        } else {
                            None
                        }
                    },
                }
            },
        ))
        .await
        .into_iter()
        .flatten()
        .min();

        if let Some(min_failed) = min_failed {
            account.set_sequence_number(min_failed);
            if start_seq_num == account.sequence_number() {
                tokio::time::sleep(poll_interval).await;
                continue;
            }
        }

        let submitted_time = tracking.submitted(txns.len(), before_submitted_instant);
        let end_onchain_ts = txns
            .iter()
            .map(|txn| txn.expiration_timestamp_secs())
            .max()
            .unwrap();

        let mut max_seen = start_seq_num;
        loop {
            match get_account_seq_num(client, account.address()).await {
                Ok((seq_num, onchain_ts)) => {
                    if seq_num > max_seen {
                        consecutive_rollback = 0;
                        tracking
                            .committed_succesfully((seq_num - max_seen) as usize, submitted_time);
                        max_seen = seq_num;
                    }

                    if seq_num == account.sequence_number() {
                        break;
                    }
                    assert!(
                        seq_num < account.sequence_number(),
                        "seq_num: {}, account.seq_num: {}",
                        seq_num,
                        account.sequence_number()
                    );

                    if onchain_ts > end_onchain_ts {
                        consecutive_rollback += 1;

                        sample!(
                            SampleRate::Duration(Duration::from_secs(10)),
                            warn!("Rolling back account {} seq num from {} to {} (cur fetched {}). {} > {}. Consecutive rollback index {}", account.address(), account.sequence_number(), max_seen, seq_num, onchain_ts, end_onchain_ts, consecutive_rollback)
                        );
                        account.set_sequence_number(max_seen);
                        if let Some(txn) = txns.iter().find(|txn| txn.sequence_number() == max_seen)
                        {
                            match client
                                .get_transaction_by_hash_bcs(txn.clone().committed_hash())
                                .await
                            {
                                Ok(res) => {
                                    sample!(
                                        SampleRate::Duration(Duration::from_secs(1)),
                                        warn!("Rollback txn status: {:?}", res.into_inner())
                                    );
                                },
                                Err(RestError::Api(AptosErrorResponse {
                                    error:
                                        AptosError {
                                            error_code: AptosErrorCode::TransactionNotFound,
                                            ..
                                        },
                                    ..
                                })) => {
                                    // no info to show
                                },
                                Err(e) => {
                                    sample!(
                                        SampleRate::Duration(Duration::from_secs(1)),
                                        warn!("Rollback error status: {:?}", e)
                                    );
                                },
                            }
                        }
                        if consecutive_rollback >= 10 {
                            sample!(
                                SampleRate::Duration(Duration::from_secs(1)),
                                warn!(
                                    "Too many consecutive rollbacks. Aborting {}",
                                    account.address()
                                )
                            );
                            break 'outer;
                        }
                        break;
                    }
                },
                Err(e) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(1)),
                        warn!("Error getting account seq num: {:?}", e)
                    );
                },
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
    if indexer_backoffs > 0 || blockchain_backoffs > 0 {
        warn!(
            "Applied {} blockchain and {} indexer backoffs",
            blockchain_backoffs, indexer_backoffs
        );
    }
}

async fn fetch_work_txn_output<T: Clone>(
    account: &LocalAccount,
    initial_seq_num: u64,
    work: &[T],
    clients: &[Client],
    progress: &AtomicUsize,
) -> Vec<(T, Option<TransactionOnChainData>)> {
    tokio::time::sleep(start_sleep_duration()).await;

    let mut start = initial_seq_num;
    let mut out = vec![];
    loop {
        let client = pick_client(clients);
        match client
            .get_account_transactions_bcs(account.address(), Some(start), None)
            .await
        {
            Ok(transactions) => {
                for txn in transactions.inner().iter() {
                    out.push((work[out.len()].clone(), Some(txn.clone())));
                }

                let len = transactions.inner().len();
                progress.fetch_add(len, std::sync::atomic::Ordering::Relaxed);

                start += len as u64;
                if start >= account.sequence_number() {
                    break;
                }
                if len == 0 {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(1)),
                        error!(
                            "Account {} seq num {}..{} (work len {}), no more transasctions fetched at {}",
                            account.address(),
                            initial_seq_num,
                            account.sequence_number(),
                            work.len(),
                            start,
                        ),
                    );

                    break;
                }
            },
            Err(e) => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(1)),
                    warn!("Error getting account transactions: {:?}", e)
                );
            },
        }
    }

    while out.len() < work.len() {
        out.push((work[out.len()].clone(), None));
    }

    out
}

pub fn create_sample_addresses(args: CreateSampleAddresses) -> Result<()> {
    let sample_address = AccountAddress::from_str_strict(
        "0xabcd000000000000000000000000000000000000000000000000000000000000",
    )?;
    let addresses = (0..args.num_addresses)
        .map(|mut i| {
            let mut vec = sample_address.into_bytes();
            let mut index = 20;
            while i > 0 {
                vec[index] = (i % 256) as u8;
                i /= 256;
                index -= 1;
            }
            AccountAddress::new(vec).to_standard_string()
        })
        .collect::<Vec<_>>();

    std::fs::write(args.output_file, addresses.join("\n"))?;
    Ok(())
}

pub fn pick_client(clients: &[Client]) -> &Client {
    clients.choose(&mut rand::thread_rng()).unwrap()
}
