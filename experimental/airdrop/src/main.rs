// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{indexer::continuously_update_indexer_delay, metrics::{spawn_async_tracking, Tracking}};
use anyhow::{bail, Context, Result};
use aptos_config::{config::DEFAULT_MAX_SUBMIT_TRANSACTION_BATCH_SIZE, keys::ConfigKey};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_logger::{info, sample, sample::SampleRate, warn, Level, Logger, error};
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    rest_client::{aptos_api_types::{AptosError, AptosErrorCode, TransactionOnChainData}, error::{AptosErrorResponse, RestError}, Client},
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{AccountKey, LocalAccount},
};
use aptos_transaction_emitter_lib::{
    emitter::{
        account_minter::{
            bulk_create_accounts, gen_reusable_accounts, prompt_yes, BulkAccountCreationConfig,
        },
        get_account_seq_num, get_needed_balance_per_account, load_specific_account, parse_seed,
        transaction_executor::RestApiReliableTransactionSubmitter,
        EXPECTED_GAS_PER_ACCOUNT_CREATE, EXPECTED_GAS_PER_TRANSFER,
    },
    Cluster, ClusterArgs,
};
use aptos_transaction_generator_lib::ReliableTransactionSubmitter;
use clap::{Parser, Subcommand};
use futures::{future::join_all, StreamExt};
use rand::{rngs::StdRng, seq::SliceRandom, thread_rng, Rng, SeedableRng};
use std::{
    collections::{BTreeMap, HashSet},
    sync::{
        atomic::{AtomicI64, AtomicUsize},
        Arc,
    },
    time::{Duration, Instant},
};
use workloads::{
    create_account_address_pairs_work, create_account_addresses_work,
    CreateAndTransferAptSignedTransactionBuilder, NftBurnSignedTransactionBuilder,
    NftMintSignedTransactionBuilder, SignedTransactionBuilder, TransferAptSignedTransactionBuilder,
};

mod indexer;
mod metrics;
mod workloads;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: AirdropCommand,
}

#[derive(Subcommand, Debug)]
enum AirdropCommand {
    Submit(Submit),
    CreateSampleAddresses(CreateSampleAddresses),
    CleanAddresses(CleanAddresses),
}

#[derive(Subcommand, Debug)]
pub enum WorkTypeSubcommand {
    TransferApt(DestinationsArg),
    CreateAndTransferApt(DestinationsArg),
    AirdropNftMint(AirdropNftArgs),
    AirdropNftBurn(AirdropNftArgs),
    ReturnWorkerFunds,
}

#[derive(Parser, Debug)]
pub struct DestinationsArg {
    #[clap(long)]
    destinations_file: String,
}

#[derive(Parser, Debug)]
pub struct AirdropNftArgs {
    #[clap(long)]
    destinations_file: String,

    /// Ed25519PrivateKey for minting coins
    #[clap(long, value_parser = ConfigKey::<Ed25519PrivateKey>::from_encoded_string)]
    pub admin_key: ConfigKey<Ed25519PrivateKey>,
}

#[derive(Debug, Parser)]
pub struct TransactionFactoryArgs {
    #[clap(long, default_value = "100")]
    gas_price: u64,

    #[clap(long)]
    init_gas_price: Option<u64>,

    #[clap(long, default_value = "10000")]
    max_gas_per_txn: u64,

    #[clap(long)]
    octas_per_workload_transaction: u64,
}

impl TransactionFactoryArgs {
    fn with_init_params(&self, factory: TransactionFactory) -> TransactionFactory {
        factory
            .with_gas_unit_price(self.init_gas_price.unwrap_or(self.gas_price))
            .with_max_gas_amount(self.max_gas_per_txn)
    }

    fn with_params(&self, factory: TransactionFactory) -> TransactionFactory {
        factory
            .with_gas_unit_price(self.gas_price)
            .with_max_gas_amount(self.max_gas_per_txn)
    }
}

#[derive(Parser, Debug)]
struct Submit {
    #[clap(flatten)]
    cluster_args: ClusterArgs,

    #[clap(flatten)]
    transaction_factory_args: TransactionFactoryArgs,

    /// Number of accounts to create
    #[clap(long)]
    num_worker_accounts: usize,

    /// Optional seed for accounts used. If no seed is provided, a random seed is used and printed.
    #[clap(long)]
    accounts_seed: Option<String>,

    #[clap(long)]
    skip_funding_accounts: bool,

    #[clap(long, default_value = "10")]
    batch_size: usize,
    #[clap(long, default_value = "60")]
    expiration_time_s: u64,
    #[clap(long, default_value = "0.3")]
    poll_interval_s: f32,

    #[clap(subcommand)]
    work_args: WorkTypeSubcommand,

    #[clap(long)]
    output_file: Option<String>,
}

#[derive(Parser, Debug)]
struct CreateSampleAddresses {
    /// Number of accounts to create
    #[clap(long)]
    num_addresses: usize,

    #[clap(long)]
    output_file: String,
}

#[derive(Parser, Debug)]
struct CleanAddresses {
    #[clap(long)]
    destinations_file: String,

    #[clap(long)]
    output_file: String,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    Logger::builder().level(Level::Info).build();

    let args = Args::parse();

    match args.command {
        AirdropCommand::Submit(args) => create_work_and_execute(args).await,
        AirdropCommand::CreateSampleAddresses(args) => create_sample_addresses(args),
        AirdropCommand::CleanAddresses(args) => clean_addresses(args),
    }
}

async fn create_work_and_execute(args: Submit) -> Result<()> {
    let cluster = Cluster::try_from_cluster_args(&args.cluster_args)
        .await
        .context("Failed to build cluster")?;
    let coin_source_account = cluster
        .load_coin_source_account(&cluster.random_instance().rest_client())
        .await?;

    match &args.work_args {
        WorkTypeSubcommand::AirdropNftMint(mint_args) => {
            let work = create_account_addresses_work(&mint_args.destinations_file, false)?;

            let client = &cluster.random_instance().rest_client();
            let admin_account = load_specific_account(
                AccountKey::from_private_key(mint_args.admin_key.private_key()),
                false,
                client,
            )
            .await?;

            let txn_factory = args.transaction_factory_args.with_init_params(
                TransactionFactory::new(cluster.chain_id)
                    .with_transaction_expiration_time(args.expiration_time_s));

            let builder =
                NftMintSignedTransactionBuilder::new(admin_account, client, txn_factory.clone())
                    .await?;
            execute_submit(work, args, builder, cluster, coin_source_account).await
        },
        WorkTypeSubcommand::AirdropNftBurn(burn_args) => {
            let work = create_account_address_pairs_work(&burn_args.destinations_file, true).await?;

            let client = &cluster.random_instance().rest_client();
            let admin_account = load_specific_account(
                AccountKey::from_private_key(burn_args.admin_key.private_key()),
                false,
                client,
            )
            .await?;

            let builder = NftBurnSignedTransactionBuilder::new(admin_account, cluster.chain_id)?;
            execute_submit(work, args, builder, cluster, coin_source_account).await
        },
        WorkTypeSubcommand::TransferApt(destinations) => {
            let work = create_account_addresses_work(&destinations.destinations_file, false)?;
            execute_submit(
                work,
                args,
                TransferAptSignedTransactionBuilder,
                cluster,
                coin_source_account,
            )
            .await
        },
        WorkTypeSubcommand::CreateAndTransferApt(destinations) => {
            let work = create_account_addresses_work(&destinations.destinations_file, false)?;
            execute_submit(
                work,
                args,
                CreateAndTransferAptSignedTransactionBuilder,
                cluster,
                coin_source_account,
            )
            .await
        },
        WorkTypeSubcommand::ReturnWorkerFunds => {
            execute_return_worker_funds(args, cluster, &coin_source_account).await
        },
    }
}

fn pick_client(clients: &Vec<Client>) -> &Client {
    clients.choose(&mut rand::thread_rng()).unwrap()
}

async fn execute_submit<T: Clone, B: SignedTransactionBuilder<T>>(
    work: Vec<T>,
    args: Submit,
    builder: B,
    cluster: Cluster,
    coin_source_account: LocalAccount,
) -> Result<()> {
    let clients = cluster
        .all_instances()
        .map(|i| i.rest_client())
        .collect::<Vec<_>>();
    let txn_factory = TransactionFactory::new(cluster.chain_id)
        .with_transaction_expiration_time(args.expiration_time_s);

    let needed_balance_per_account = get_needed_balance_per_account(
        work.len() as u64,
        0,
        args.transaction_factory_args.octas_per_workload_transaction,
        args.num_worker_accounts,
        args.transaction_factory_args.gas_price,
        args.transaction_factory_args.max_gas_per_txn,
    );

    let worker_accounts = create_worker_accounts(
        clients.clone(),
        coin_source_account,
        args.transaction_factory_args
            .with_init_params(txn_factory.clone()),
        args.num_worker_accounts,
        args.accounts_seed.as_deref(),
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
        Duration::from_secs_f32(args.poll_interval_s),
        args.transaction_factory_args.with_params(txn_factory),
        builder,
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

async fn execute_return_worker_funds(
    args: Submit,
    cluster: Cluster,
    coin_source_account: &LocalAccount,
) -> Result<()> {
    let return_funds_retries = 5;
    let return_funds_retry_interval = Duration::from_secs(3);

    let clients = cluster
        .all_instances()
        .map(|i| i.rest_client())
        .collect::<Vec<_>>();

    let txn_factory = args.transaction_factory_args.with_params(
        TransactionFactory::new(cluster.chain_id)
            .with_transaction_expiration_time(args.expiration_time_s),
    );

    let txn_executor = RestApiReliableTransactionSubmitter::new(
        clients,
        return_funds_retries,
        return_funds_retry_interval,
    );
    let accounts = gen_reusable_accounts(
        &txn_executor,
        args.num_worker_accounts,
        &mut StdRng::from_seed(parse_seed(&args.accounts_seed.unwrap())),
    )
    .await?;

    let txn_executor_ref = &txn_executor;
    let counter = txn_executor_ref.create_counter_state();
    let counter_ref = &counter;
    let txn_factory_ref = &txn_factory;
    let _ = futures::stream::iter(accounts.iter().map(|account| async move {
        loop {
            if let Ok(balance) = txn_executor_ref
                .get_account_balance(account.address())
                .await
            {
                if balance > 0 {
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
                }
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
        &coin_source_account,
        &RestApiReliableTransactionSubmitter::new(
            clients,
            account_funding_retries,
            account_funding_retry_interval,
        ),
        &init_txn_factory,
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

async fn execute_txn_list<T: Clone, B: SignedTransactionBuilder<T>>(
    accounts: Vec<LocalAccount>,
    clients: Vec<Client>,
    work: Vec<T>,
    batch_size: usize,
    poll_interval: Duration,
    txn_factory: TransactionFactory,
    builder: B,
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
    let tracking = Arc::new(Tracking::new());
    let tracking_ref = tracking.as_ref();

    let tracking_done = spawn_async_tracking(tracking.clone(), Duration::from_secs(2));

    let start_time = Instant::now();
    let indexer_delay = Arc::new(AtomicI64::new(0));
    let _task = tokio::spawn(continuously_update_indexer_delay(
        txn_factory.get_chain_id(),
        indexer_delay.clone(),
    ));
    let indexer_delay_ref = &indexer_delay;

    join_all(
        accounts_with_work
            .iter()
            .map(|account_with_work| async move {
                submit_work_txns(
                    &account_with_work.account,
                    account_with_work.initial_seq_num,
                    &account_with_work.work,
                    batch_size,
                    builder,
                    txn_factory,
                    clients,
                    poll_interval,
                    tracking_ref,
                    indexer_delay_ref,
                )
                .await;
            }),
    )
    .await;

    let elapsed = start_time.elapsed().as_secs_f64();
    tracking_done.store(true, std::sync::atomic::Ordering::Relaxed);
    tokio::time::sleep(Duration::from_secs(1)).await;

    info!("Done executing work");
    tracking.print_stats(elapsed);

    let progress = Arc::new(AtomicUsize::new(0));
    let done_tracking = spawn_async_tracking(progress.clone(), Duration::from_secs(10));
    let progress_ref = progress.as_ref();
    let out = futures::stream::iter(accounts_with_work.iter().map(
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

    for (status, infos) in group_pairs(out.iter().map(|txn| (txn.1.as_ref().map_or("missing".to_string(), |t| format!("{:?}", t.info.status())), txn))).into_iter()
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

async fn submit_work_txns<T, B: SignedTransactionBuilder<T>>(
    account: &LocalAccount,
    initial_seq_num: u64,
    work: &Vec<T>,
    batch_size: usize,
    builder: &B,
    txn_factory: &TransactionFactory,
    clients: &Vec<Client>,
    poll_interval: Duration,
    tracking: &Tracking,
    indexer_delay_ref: &AtomicI64,
) {
    tokio::time::sleep(Duration::from_secs_f64(
        rand::thread_rng().gen_range(0.0, 5.0),
    ))
    .await;

    let mut consecutive_rollback = 0;

    loop {
        let indexer_lower_threshold = 10.0;
        let indexer_threshold_gap = 25.0;
        let indexer_delay = indexer_delay_ref.load(std::sync::atomic::Ordering::Relaxed) as f64;
        if indexer_delay > indexer_lower_threshold {
            // the bigger the delay, the more likely we should wait
            // if delay is above 20s, we completely pause the submission
            if thread_rng().gen_bool(((indexer_delay - indexer_lower_threshold) / indexer_threshold_gap).sqrt().min(1.0)) {
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        }

        let blockchain_lower_theshold = 10.0;
        let blockchain_threshold_gap = 20.0;
        let last_blockchain_latency = tracking.get_last_latency();
        if last_blockchain_latency > blockchain_lower_theshold {
            // the bigger the delay, the more likely we should wait
            // if delay is above 20s, we completely pause the submission
            if thread_rng().gen_bool(((last_blockchain_latency - blockchain_lower_theshold) / blockchain_threshold_gap).sqrt().min(1.0)) {
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        }
        let start_seq_num = account.sequence_number();
        let chunk_start = (start_seq_num - initial_seq_num) as usize;
        if chunk_start >= work.len() {
            break;
        }
        let chunk = &work[chunk_start..(work.len().min(chunk_start + batch_size))];

        let txns = chunk
            .iter()
            .map(|data| builder.build(data, account, txn_factory))
            .collect::<Vec<_>>();

        let client = pick_client(clients);
        match client.submit_batch_bcs(&txns).await {
            Err(e) => {
                warn!("Error submitting batch: {:?}", e);
                continue;
            },
            Ok(r) => {
                let result = r.into_inner();
                if !result.transaction_failures.is_empty() {
                    warn!("Failed submission: {:?}", result.transaction_failures);
                }
            },
        }
        let submitted_time = tracking.submitted(txns.len());
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
                                Err(RestError::Api(AptosErrorResponse{error: AptosError{error_code: AptosErrorCode::TransactionNotFound, ..}, ..})) => {
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
                                warn!("Too many consecutive rollbacks. Aborting {}", account.address())
                            );
                            return;
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
}

async fn fetch_work_txn_output<T: Clone>(
    account: &LocalAccount,
    initial_seq_num: u64,
    work: &[T],
    clients: &Vec<Client>,
    progress: &AtomicUsize,
) -> Vec<(T, Option<TransactionOnChainData>)> {
    tokio::time::sleep(Duration::from_secs_f64(
        rand::thread_rng().gen_range(0.0, 5.0),
    ))
    .await;

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

fn create_sample_addresses(args: CreateSampleAddresses) -> Result<()> {
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

fn clean_addresses(args: CleanAddresses) -> Result<()> {
    let work = create_account_addresses_work(&args.destinations_file, false)?;
    println!("Input: {}", work.len());
    let mut unique = work.into_iter().collect::<HashSet<_>>().into_iter().collect::<Vec<_>>();
    unique.shuffle(&mut thread_rng());
    println!("Output: {}", unique.len());
    std::fs::write(args.output_file, unique.iter().map(AccountAddress::to_standard_string).collect::<Vec<_>>().join("\n"))?;
    Ok(())
}
