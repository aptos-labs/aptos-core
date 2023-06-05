// Copyright © Aptos Foundation

use anyhow::{bail, Context, Result};
use aptos_indexer_grpc_cache_worker::IndexerGrpcCacheWorkerConfig;
use aptos_indexer_grpc_file_store::IndexerGrpcFileStoreWorkerConfig;
use aptos_indexer_grpc_server_framework::{
    run_server_with_config, setup_logging, setup_panic_handler, GenericConfig, RunnableConfig,
};
use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    config::{IndexerGrpcFileStoreConfig, LocalFileStore},
    constants::BLOB_STORAGE_SIZE,
    file_store_operator::{FileStoreOperator, LocalFileStoreOperator},
};
use aptos_transaction_emitter_lib::{emit_transactions, ClusterArgs, CoinSourceArgs, EmitArgs};
use aptos_transaction_generator_lib::args::TransactionTypeArg;
use aptos_types::chain_id::ChainId;
use regex::Regex;
use std::{fs::File, io::Write, path::PathBuf};
use tempfile::TempDir;
use tokio::task::JoinHandle;
use tracing::info;

static TESTNET_REST_API_URL: &str = "http://localhost:8080";
static TESTNET_FULLNODE_GRPC_URL: &str = "localhost:50051";
static REDIS_PRIMARY_URL: &str = "localhost:6379";

static MINT_KEY_FILE_NAME: &str = "mint.key";

/// Get the name of docker containers that match the given regex
/// This works around different docker compose v1 and v2 naming conventions
fn get_container_by_name_regex(name_regex: Regex) -> Result<Vec<String>> {
    let containers = std::process::Command::new("docker")
        .args(&["ps", "--format", "{{.Names}}"])
        .output()?;
    let containers = String::from_utf8(containers.stdout)?;
    let ret = containers
        .split("\n")
        .map(|x| x.to_string())
        .filter(|x| name_regex.is_match(x))
        .collect::<Vec<String>>();
    Ok(ret)
}

/// Connects to the local redis running in docker and resets it
async fn reset_redis() -> Result<()> {
    let redis_containers = get_container_by_name_regex(Regex::new(r".*redis.*")?)?;
    for container in redis_containers {
        let _ = std::process::Command::new("docker")
            .args(&["exec", &container, "redis-cli", "FLUSHALL"])
            .output()?;
    }

    let conn = redis::Client::open(format!("redis://{}", REDIS_PRIMARY_URL))
        .expect("Create redis client failed.")
        .get_async_connection()
        .await
        .expect("Create redis connection failed.");
    let mut cache_operator = CacheOperator::new(conn);
    match cache_operator.get_latest_version().await {
        Ok(x) => {
            bail!(
                "Redis did not scale down properly. There's still stuff in the cache. Latest version: {}",
                x
            );
        },
        Err(_) => info!("Redis scaled down properly"),
    }
    Ok(())
}

/// Fetch the mint key from the running local testnet and dump it into the path specified
async fn dump_mint_key_to_file(path: &PathBuf) -> Result<String> {
    let validator_containers =
        get_container_by_name_regex(Regex::new(r"validator-testnet.*validator.*")?)?;
    if validator_containers.len() != 1 {
        bail!(
            "Expected 1 validator container, found {}",
            validator_containers.len()
        );
    }
    let validator = &validator_containers[0];
    let output = std::process::Command::new("docker")
        .args(&["exec", validator, "cat", "/opt/aptos/var/mint.key"])
        .output()?;
    let output_stdout = output.stdout;
    info!("Mint key: {:?}", output_stdout);
    let mint_key_path = path.join(MINT_KEY_FILE_NAME);
    let mint_key_path_string = mint_key_path.display().to_string();
    let mut file = File::create(mint_key_path).context("Could not create mint key in path")?;
    file.write_all(&output_stdout)
        .context("Could not write mint key to file")?;
    Ok(mint_key_path_string)
}

/// Emit transactions to the local testnet to invoke certain indexer actions, such as writing
/// to filestore
async fn emit_transactions_for_test() -> Result<()> {
    // dump the key to a tempfile
    let path_buf = TempDir::new()
        .context("Could not create temp dir")?
        .into_path();
    let mint_key_file_path = dump_mint_key_to_file(&path_buf)
        .await
        .expect("Failed to fetch mint key");
    info!("Mint key file path: {}", mint_key_file_path);

    // emit some transactions
    let duration = 10;
    let target_tps = BLOB_STORAGE_SIZE / duration;
    let cluster_args = ClusterArgs {
        targets: Some(vec![url::Url::parse(TESTNET_REST_API_URL)
            .context("Cannot parse default fullnode url")
            .unwrap()]),
        targets_file: None,
        reuse_accounts: false,
        chain_id: ChainId::test(),
        coin_source_args: CoinSourceArgs {
            mint_file: Some(mint_key_file_path),
            ..CoinSourceArgs::default()
        },
    };
    let emit_args = EmitArgs {
        // mempool_backlog: None,
        target_tps: Some(target_tps),
        txn_expiration_time_secs: 30,
        duration: duration.try_into().unwrap(),
        transaction_type: vec![TransactionTypeArg::default()],
        ..EmitArgs::default()
    };

    info!(
        "Emitting transactions: {} tps for {} seconds...",
        target_tps, duration
    );

    let stats = emit_transactions(&cluster_args, &emit_args)
        .await
        .map_err(|e| panic!("Emit transactions failed {:?}", e))
        .unwrap();
    info!("Total stats: {}", stats);
    info!("Average rate: {}", stats.rate());
    Ok(())
}

async fn start_server<T: RunnableConfig>(
    server_config: T,
) -> Result<(u16, JoinHandle<Result<()>>)> {
    let health_check_port = aptos_config::utils::get_available_port();
    let config = GenericConfig {
        health_check_port,
        server_config,
    };
    let server_name = config.server_config.get_server_name();
    info!(
        "Starting server {} with healtheck port {}",
        server_name, health_check_port
    );
    let runtime_handle = tokio::runtime::Handle::current();

    // runs the component's run, but we need the server run
    let join_handle = runtime_handle.spawn(async move { run_server_with_config(config).await });
    let startup_timeout_secs = 30;
    for i in 0..startup_timeout_secs {
        match reqwest::get(format!("http://localhost:{}/metrics", health_check_port)).await {
            Ok(_) => break,
            Err(e) => {
                if i == startup_timeout_secs - 1 {
                    let msg = if join_handle.is_finished() {
                        format!("Server failed on startup: {:#?}", join_handle.await)
                    } else {
                        "Server was still starting up".to_string()
                    };
                    bail!(
                        "Server didn't come up within given timeout: {:#?} {}",
                        e,
                        msg
                    );
                }
            },
        }
        if join_handle.is_finished() {
            bail!(
                "Server returned error while starting up: {:#?}",
                join_handle.await
            );
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Ok((health_check_port, join_handle))
}

async fn setup_test() {
    // we're going to run both cache worker and file store worker in the same process
    // so we need to centrally set up logging and panic handler, whereas they are usually done in the same service
    setup_logging();
    setup_panic_handler();
    reset_redis().await.expect("Failed to reset redis for test");

    // aptos_logger too
    aptos_logger::Logger::init_for_testing();
}

// These tests expect that the local environment has a running fullnode
// This can be done by using the docker-compose
// We will then simulate chaos by using (1) docker exec (2) docker-compose scale <service>=<num_replicas>
#[tokio::test]
pub async fn verify_docker_compose_setup() {
    reqwest::get(&format!("{}/v1", TESTNET_REST_API_URL))
        .await
        .unwrap()
        .error_for_status()
        .unwrap(); // we just want a good status code
}

/// Test that the cache worker can start from scratch and make progress.
/// This is a cold start because there is no existing cache and also it is unable to read from the file store
/// about the latest state prior to starting.
#[tokio::test]
async fn test_cold_start_cache_worker_progress() {
    setup_test().await;

    let tmp_dir = TempDir::new().expect("Could not create temp dir"); // start with a new file store each time
    let cache_worker_config = IndexerGrpcCacheWorkerConfig {
        fullnode_grpc_address: TESTNET_FULLNODE_GRPC_URL.to_string(),
        file_store_config: IndexerGrpcFileStoreConfig::LocalFileStore(LocalFileStore {
            local_file_store_path: tmp_dir.path().to_path_buf(),
        }),
        redis_main_instance_address: REDIS_PRIMARY_URL.to_string(),
    };

    let (_cache_worker_port, _cache_worker_handle) =
        start_server::<IndexerGrpcCacheWorkerConfig>(cache_worker_config)
            .await
            .expect("Failed to start CacheWorker");

    let conn = redis::Client::open(format!("redis://{}", REDIS_PRIMARY_URL.to_string()))
        .expect("Create redis client failed.")
        .get_async_connection()
        .await
        .expect("Create redis connection failed.");

    let check_cache_secs = 30;
    let check_cache_frequency_secs = 5;
    let tries = check_cache_secs / check_cache_frequency_secs;

    // check that the cache was written to
    let mut cache_operator = CacheOperator::new(conn);
    let mut chain_id = 0;
    for _ in 0..tries {
        match cache_operator.get_chain_id().await {
            Ok(x) => {
                chain_id = x;
                info!("Chain id: {}", x);
                break;
            },
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(check_cache_frequency_secs))
                    .await;
            },
        }
    }
    assert!(chain_id == 4);

    // check that the cache worker is making progress
    let mut latest_version = 0;
    let mut new_latest_version;
    for _ in 0..tries {
        tokio::time::sleep(std::time::Duration::from_secs(check_cache_frequency_secs)).await;
        new_latest_version = cache_operator.get_latest_version().await.unwrap();
        info!(
            "Processed {} versions since last check {}s ago...",
            new_latest_version - latest_version,
            check_cache_frequency_secs
        );
        assert!(new_latest_version > latest_version);
        latest_version = new_latest_version;
    }
}

/// Test that the file store worker can start from scratch and make progress.
/// This is a cold start since the file store and the cache start as empty. And the file store is generally the source of truth
/// between the two. We expect the file store to be written to
#[tokio::test]
async fn test_cold_start_file_store_worker_progress() {
    setup_test().await;

    let tmp_dir = TempDir::new().expect("Could not create temp dir"); // start with a new file store each time

    let cache_worker_config = IndexerGrpcCacheWorkerConfig {
        fullnode_grpc_address: TESTNET_FULLNODE_GRPC_URL.to_string(),
        file_store_config: IndexerGrpcFileStoreConfig::LocalFileStore(LocalFileStore {
            local_file_store_path: tmp_dir.path().to_path_buf(),
        }),
        redis_main_instance_address: REDIS_PRIMARY_URL.to_string(),
    };

    let file_store_worker_config = IndexerGrpcFileStoreWorkerConfig {
        redis_main_instance_address: REDIS_PRIMARY_URL.to_string(),
        file_store_config: IndexerGrpcFileStoreConfig::LocalFileStore(LocalFileStore {
            local_file_store_path: tmp_dir.path().to_path_buf(),
        }),
    };

    let (_cache_worker_port, _cache_worker_handle) =
        start_server::<IndexerGrpcCacheWorkerConfig>(cache_worker_config.clone())
            .await
            .expect("Failed to start CacheWorker");

    // XXX: wait some time before file store starts up. we should resolve the boot dependency cycle
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    let (_file_store_port, _file_store_handle) =
        start_server::<IndexerGrpcFileStoreWorkerConfig>(file_store_worker_config)
            .await
            .expect("Failed to start FileStoreWorker");

    // wait until file store writes its first metadata
    let file_store_operator = LocalFileStoreOperator::new(tmp_dir.path().to_path_buf());
    let tries = 6;
    for _ in 0..tries {
        match file_store_operator.get_file_store_metadata().await {
            Some(_) => {
                info!("File store metadata found");
                break;
            },
            None => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            },
        }
    }

    // inspect the files at boot. there should at least be the metadata file specifying that it has 0 versions processed
    info!(
        "Expecting file store to have files {}",
        tmp_dir.path().display()
    );
    let file_store_metadata = file_store_operator.get_file_store_metadata().await;
    assert!(file_store_metadata.is_some());

    // emit transactions, enough to write to file store
    emit_transactions_for_test()
        .await
        .expect("Emit transactions failed");

    // after a while, expect the metadata file to be updated with the latest version
    let file_store_metadata = file_store_operator
        .get_file_store_metadata()
        .await
        .expect("Failed to get file store metadata");
    info!(
        "[Indexer Cache] File store metadata: {:?}",
        file_store_metadata
    );
    assert!(file_store_metadata.version > 0);
}
