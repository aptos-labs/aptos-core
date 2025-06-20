// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(unexpected_cfgs)]
#![forbid(unsafe_code)]

mod consensus;
mod indexer;
mod logger;
mod network;
mod services;
mod state_sync;
mod storage;
pub mod utils;

#[cfg(test)]
mod tests;

use crate::utils::ensure_max_open_files_limit;
use anyhow::{anyhow, Context};
use aptos_admin_service::AdminService;
use aptos_api::bootstrap as bootstrap_api;
use aptos_build_info::build_information;
use aptos_config::config::{merge_node_config, NodeConfig, PersistableConfig};
use aptos_framework::ReleaseBundle;
use aptos_genesis::builder::GenesisConfiguration;
use aptos_logger::{prelude::*, telemetry_log_writer::TelemetryLog, Level, LoggerFilterUpdater};
use aptos_state_sync_driver::driver_factory::StateSyncRuntimes;
use aptos_types::{
    chain_id::ChainId, keyless::Groth16VerificationKey, on_chain_config::OnChainJWKConsensusConfig,
};
use clap::Parser;
use futures::channel::{mpsc, oneshot};
use hex::{FromHex, FromHexError};
use rand::{rngs::StdRng, SeedableRng};
use serde_json::Value;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use tokio::runtime::Runtime;

const EPOCH_LENGTH_SECS: u64 = 60;

/// Runs an Aptos validator or fullnode
#[derive(Clone, Debug, Parser)]
#[clap(name = "Aptos Node", author, version)]
pub struct AptosNodeArgs {
    /// Path to node configuration file (or template for local test mode).
    #[clap(
        short = 'f',
        long,
        value_parser,
        required_unless_present_any = ["test", "info"],
    )]
    #[cfg_attr(target_os = "linux", clap(required_unless_present_any = ["stacktrace"]))]
    config: Option<PathBuf>,

    /// Directory to run the test mode in.
    ///
    /// Repeated runs will start up from previous state.
    #[clap(long, value_parser, requires("test"))]
    test_dir: Option<PathBuf>,

    /// Path to node configuration file override for local test mode. Cannot be used with --config
    #[clap(long, value_parser, requires("test"), conflicts_with("config"))]
    test_config_override: Option<PathBuf>,

    /// Run only a single validator node testnet.
    #[clap(long)]
    test: bool,

    /// Optimize the single validator node testnet for higher performance
    #[clap(long, requires("test"))]
    performance: bool,

    /// Random number generator seed for starting a single validator testnet.
    #[clap(long, value_parser = load_seed, requires("test"))]
    seed: Option<[u8; 32]>,

    /// Use random ports instead of ports from the node configuration.
    #[clap(long, requires("test"))]
    random_ports: bool,

    /// Paths to the Aptos framework release package to be used for genesis.
    #[clap(long, requires("test"))]
    genesis_framework: Option<PathBuf>,

    /// Enable lazy mode.
    ///
    /// Setting this flag will set `consensus#mempool_poll_count` config to `u64::MAX` and
    /// only commit a block when there are user transactions in mempool.
    #[clap(long, requires("test"))]
    lazy: bool,

    /// Display information about the build of this node
    #[clap(long)]
    info: bool,

    #[cfg(target_os = "linux")]
    /// Start as a child process to collect thread dump.
    /// See rstack-self crate for more details.
    #[clap(long)]
    stacktrace: bool,
}

impl AptosNodeArgs {
    /// Runs an Aptos node based on the given command line arguments and config flags
    pub fn run(self) {
        #[cfg(target_os = "linux")]
        // https://sfackler.github.io/rstack/doc/rstack_self/index.html
        //
        // TODO(grao): I don't like this way, but I didn't find other existing solution in Rust.
        // Maybe try to use libc directly?
        if self.stacktrace {
            let _ = rstack_self::child();
            return;
        }

        if self.info {
            let build_information = build_information!();
            println!(
                "{}",
                serde_json::to_string_pretty(&build_information)
                    .expect("Failed to print build information")
            );
            return;
        }

        if self.test {
            println!("WARNING: Entering test mode! This should never be used in production!");
            if self.performance {
                println!("WARNING: Entering performance mode! System utilization may be high!");
            }

            // Set the genesis framework
            let genesis_framework = if let Some(path) = self.genesis_framework {
                ReleaseBundle::read(path).unwrap()
            } else {
                aptos_cached_packages::head_release_bundle().clone()
            };

            // Create a seeded RNG, setup the test environment and start the node
            let rng = self
                .seed
                .map(StdRng::from_seed)
                .unwrap_or_else(StdRng::from_entropy);
            setup_test_environment_and_start_node(
                &self.config,
                &self.test_config_override,
                None,
                self.test_dir,
                self.random_ports,
                self.lazy,
                self.performance,
                &genesis_framework,
                rng,
            )
            .expect("Test node should start correctly!");
        } else {
            // Get the config file path
            let config_path = self.config.expect("Config is required to launch node");
            if !config_path.exists() {
                panic!(
                    "The node config file could not be found! Ensure the given path is correct: {:?}",
                    config_path.display()
                )
            }

            // A config file exists, attempt to parse the config
            let config = NodeConfig::load_from_path(config_path.clone()).unwrap_or_else(|error| {
                panic!(
                    "Failed to load the node config file! Given file path: {:?}. Error: {:?}",
                    config_path.display(),
                    error
                )
            });

            // Start the node
            start(config, None, true).expect("Node should start correctly");
        };
    }
}

pub fn load_seed(input: &str) -> Result<[u8; 32], FromHexError> {
    let trimmed_input = input.trim();
    FromHex::from_hex(trimmed_input)
}

/// Runtime handle to ensure that all inner runtimes stay in scope
pub struct AptosHandle {
    _admin_service: AdminService,
    _api_runtime: Option<Runtime>,
    _backup_runtime: Option<Runtime>,
    _consensus_observer_runtime: Option<Runtime>,
    _consensus_publisher_runtime: Option<Runtime>,
    _consensus_runtime: Option<Runtime>,
    _dkg_runtime: Option<Runtime>,
    _indexer_grpc_runtime: Option<Runtime>,
    _indexer_runtime: Option<Runtime>,
    _indexer_table_info_runtime: Option<Runtime>,
    _jwk_consensus_runtime: Option<Runtime>,
    _mempool_runtime: Runtime,
    _network_runtimes: Vec<Runtime>,
    _peer_monitoring_service_runtime: Runtime,
    _state_sync_runtimes: StateSyncRuntimes,
    _telemetry_runtime: Option<Runtime>,
    _indexer_db_runtime: Option<Runtime>,
}

pub fn start(
    config: NodeConfig,
    log_file: Option<PathBuf>,
    create_global_rayon_pool: bool,
) -> anyhow::Result<()> {
    start_and_report_ports(config, log_file, create_global_rayon_pool, None, None)
}

/// Start an Aptos node
pub fn start_and_report_ports(
    config: NodeConfig,
    log_file: Option<PathBuf>,
    create_global_rayon_pool: bool,
    api_port_tx: Option<oneshot::Sender<u16>>,
    indexer_grpc_port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<()> {
    // Setup panic handler
    aptos_crash_handler::setup_panic_handler();

    // Create global rayon thread pool
    utils::create_global_rayon_pool(create_global_rayon_pool);

    // Initialize the global aptos-node-identity
    aptos_node_identity::init(config.get_peer_id())?;

    // Instantiate the global logger
    let (remote_log_receiver, logger_filter_update) = logger::create_logger(&config, log_file);

    // Ensure `ulimit -n`.
    ensure_max_open_files_limit(
        config.storage.ensure_rlimit_nofile,
        config.storage.assert_rlimit_nofile,
    );

    assert!(
        !cfg!(feature = "testing") && !cfg!(feature = "fuzzing"),
        "Testing features shouldn't be compiled"
    );

    // Ensure failpoints are configured correctly
    if fail::has_failpoints() {
        warn!("Failpoints are enabled!");

        // Set all of the failpoints
        if let Some(failpoints) = &config.failpoints {
            for (point, actions) in failpoints {
                fail::cfg(point, actions).unwrap_or_else(|_| {
                    panic!(
                        "Failed to set actions for failpoint! Failpoint: {:?}, Actions: {:?}",
                        point, actions
                    )
                });
            }
        }
    } else if config.failpoints.is_some() {
        warn!("Failpoints is set in the node config, but the binary didn't compile with this feature!");
    }

    // Set up the node environment and start it
    let _node_handle = setup_environment_and_start_node(
        config,
        remote_log_receiver,
        Some(logger_filter_update),
        api_port_tx,
        indexer_grpc_port_tx,
    )?;
    let term = Arc::new(AtomicBool::new(false));
    while !term.load(Ordering::Acquire) {
        thread::park();
    }

    Ok(())
}

/// Load a config based on a variety of different ways to provide config options. For
/// more information about each argument and its precedence, see
/// `setup_test_environment_and_start_node`.
pub fn load_node_config<R>(
    config_path: &Option<PathBuf>,
    test_config_override_path: &Option<PathBuf>,
    test_dir: &Path,
    random_ports: bool,
    enable_lazy_mode: bool,
    enable_performance_mode: bool,
    framework: &ReleaseBundle,
    rng: R,
) -> anyhow::Result<NodeConfig>
where
    R: rand::RngCore + rand::CryptoRng,
{
    // The validator builder puts the first node in the 0 directory
    let validator_config_path = test_dir.join("0").join("node.yaml");

    let config = if validator_config_path.exists() {
        NodeConfig::load_from_path(&validator_config_path)
            .map_err(|error| anyhow!("Unable to load config: {:?}", error))?
    } else {
        // Create a test only config for a single validator node.
        let config = create_single_node_test_config(
            config_path,
            test_config_override_path,
            test_dir,
            random_ports,
            enable_lazy_mode,
            enable_performance_mode,
            framework,
            rng,
        )?;
        if let Some(ref test_config_override_path) = test_config_override_path {
            println!(
                "\tMerged default config with override from path: {:?}",
                test_config_override_path
            );
        }
        if let Some(ref config_path) = config_path {
            println!("\tUsed user-provided config from path: {:?}", config_path);
        }
        config
    };

    Ok(config)
}

/// Print details about a node config configured for a test environment and start it.
pub fn start_test_environment_node(
    config: NodeConfig,
    test_dir: PathBuf,
    enable_lazy_mode: bool,
) -> anyhow::Result<()> {
    let aptos_root_key_path = test_dir.join("mint.key");

    // Prepare log file since we cannot automatically route logs to stderr
    let log_file = test_dir.join("validator.log");

    // Print out useful information about the environment and the node
    println!("Completed generating configuration:");
    println!("\tLog file: {:?}", log_file);
    println!("\tTest dir: {:?}", test_dir);
    println!("\tAptos root key path: {:?}", aptos_root_key_path);
    println!("\tWaypoint: {}", config.base.waypoint.genesis_waypoint());
    println!("\tChainId: {}", ChainId::test().id());
    println!("\tREST API endpoint: http://{}", &config.api.address);
    println!(
        "\tMetrics endpoint: http://{}:{}/metrics",
        &config.inspection_service.address, &config.inspection_service.port
    );
    println!(
        "\tAptosnet fullnode network endpoint: {}",
        &config.full_node_networks[0].listen_address
    );
    if config.indexer_grpc.enabled {
        println!(
            "\tIndexer gRPC node stream endpoint: {}",
            config.indexer_grpc.address
        );
    }
    if enable_lazy_mode {
        println!("\tLazy mode is enabled");
    }
    println!("\nAptos is running, press ctrl-c to exit\n");

    start(config, Some(log_file), false)
}

/// Creates a simple test environment and starts the node.
///
/// You will notice many args referring to configs. Let's explain them:
/// - `test_config_override_path` is the path to a config file that will be used as
///   a template when building the final config. If not provided, a default template
///   will be used. Many overrides are applied on top of this base config.
/// - `config_path` is similar to `test_config_override_path`, but many of the
///   overrides that are applied when using `test_config_override_path` are not
///   applied when using `config_path`. Read the code for more info.
/// - `config` is a complete NodeConfig. No overrides are applied on top of this if
///    it is provided. If both `config` and `test_dir` are provided, `config` takes
///    precedence.
/// - `test_dir` is a directory that contains a config file. Much like `config`, the
///   config read from this file is used without any overrides.
pub fn setup_test_environment_and_start_node<R>(
    config_path: &Option<PathBuf>,
    test_config_override_path: &Option<PathBuf>,
    config: Option<NodeConfig>,
    test_dir: Option<PathBuf>,
    random_ports: bool,
    enable_lazy_mode: bool,
    enable_performance_mode: bool,
    framework: &ReleaseBundle,
    rng: R,
) -> anyhow::Result<()>
where
    R: rand::RngCore + rand::CryptoRng,
{
    // If there wasn't a test directory specified, create a temporary one
    let test_dir =
        test_dir.unwrap_or_else(|| aptos_temppath::TempPath::new().as_ref().to_path_buf());

    // Create the directories for the node
    fs::DirBuilder::new().recursive(true).create(&test_dir)?;
    let test_dir = test_dir.canonicalize()?;

    let config = match config {
        Some(config) => config,
        None => load_node_config(
            config_path,
            test_config_override_path,
            &test_dir,
            random_ports,
            enable_lazy_mode,
            enable_performance_mode,
            framework,
            rng,
        )?,
    };

    start_test_environment_node(config, test_dir, enable_lazy_mode)
}

/// Creates a single node test config, with a few config tweaks to reduce
/// the overhead of running the node on a local machine. It writes necessary
/// configuration artifacts (e.g. the mint key) to disk.
pub fn create_single_node_test_config<R>(
    config_path: &Option<PathBuf>,
    test_config_override_path: &Option<PathBuf>,
    test_dir: &Path,
    random_ports: bool,
    enable_lazy_mode: bool,
    enable_performance_mode: bool,
    framework: &ReleaseBundle,
    rng: R,
) -> anyhow::Result<NodeConfig>
where
    R: rand::RngCore + rand::CryptoRng,
{
    let mut node_config = match test_config_override_path {
        // If a config override path was provided, merge it with the default config
        Some(test_config_override_path) => {
            let reader = fs::File::open(test_config_override_path).map_err(|e| {
                anyhow!(
                    "Unable to open config override file {:?}. Error: {}",
                    test_config_override_path,
                    e
                )
            })?;
            let values: serde_yaml::Value = serde_yaml::from_reader(&reader).map_err(|e| {
                anyhow!(
                    "Unable to read config override file as YAML {:?}. Error: {}",
                    test_config_override_path,
                    e
                )
            })?;
            merge_node_config(NodeConfig::get_default_validator_config(), values)?
        },
        None => NodeConfig::get_default_validator_config(),
    };

    // Adjust some fields in the default template to lower the overhead of
    // running on a local machine.
    // Some are further overridden to give us higher performance when enable_performance_mode is true
    node_config
        .consensus
        .quorum_store
        .num_workers_for_remote_batches = 1;

    if enable_performance_mode {
        // Setting to a pretty conservative concurrency level. It can be tuned locally.
        node_config.execution.concurrency_level = 4;
        // Don't constrain the TPS of Quorum Store for this single node.
        node_config
            .consensus
            .quorum_store
            .back_pressure
            .dynamic_max_txn_per_s = 10_000;
    } else {
        node_config.execution.concurrency_level = 1;
        node_config.execution.num_proof_reading_threads = 1;
        node_config.consensus.quorum_store_poll_time_ms = 1000;
    }

    node_config.execution.paranoid_hot_potato_verification = false;
    node_config.execution.paranoid_type_verification = false;
    node_config
        .execution
        .processed_transactions_detailed_counters = false;

    node_config.peer_monitoring_service.max_concurrent_requests = 1;
    node_config
        .peer_monitoring_service
        .enable_peer_monitoring_client = false;

    if enable_performance_mode {
        node_config
            .mempool
            .shared_mempool_max_concurrent_inbound_syncs = 16;
        node_config.mempool.shared_mempool_tick_interval_ms = 10;
        node_config.mempool.default_failovers = 0;
    } else {
        node_config
            .mempool
            .shared_mempool_max_concurrent_inbound_syncs = 1;
        node_config.mempool.default_failovers = 1;
        node_config.mempool.max_broadcasts_per_peer = 1;
    }

    node_config
        .state_sync
        .state_sync_driver
        .enable_auto_bootstrapping = true;
    node_config
        .state_sync
        .state_sync_driver
        .max_connection_deadline_secs = 1;
    node_config
        .state_sync
        .state_sync_driver
        .progress_check_interval_ms = 10_000;
    node_config
        .state_sync
        .data_streaming_service
        .progress_check_interval_ms = 10_000;

    // Configure the validator network
    let validator_network = node_config.validator_network.as_mut().unwrap();
    validator_network.connectivity_check_interval_ms = 10000;
    validator_network.max_connection_delay_ms = 10000;
    validator_network.ping_interval_ms = 10000;
    validator_network.runtime_threads = Some(1);

    // Configure the fullnode network
    let fullnode_network = node_config.full_node_networks.get_mut(0).unwrap();
    fullnode_network.connectivity_check_interval_ms = 10000;
    fullnode_network.max_connection_delay_ms = 10000;
    fullnode_network.ping_interval_ms = 10000;
    fullnode_network.runtime_threads = Some(1);

    // If a config path was provided, use that as the template
    if let Some(config_path) = config_path {
        node_config = NodeConfig::load_config(config_path).map_err(|e| {
            anyhow!(
                "Unable to load config from path: {:?}. Error: {:?}",
                config_path,
                e
            )
        })?;
    }

    // Change the default log level
    node_config.logger.level = Level::Debug;

    // Enable the REST API
    node_config.api.address = format!("0.0.0.0:{}", node_config.api.address.port())
        .parse()
        .expect("Unable to set the REST API address!");

    // Set the correct poll count for mempool
    if enable_lazy_mode {
        node_config.consensus.quorum_store_poll_time_ms = 3_600_000;
    }

    // The validator builder puts the first node in the 0 directory
    let aptos_root_key_path = test_dir.join("mint.key");

    // Build genesis and the validator node
    let builder = aptos_genesis::builder::Builder::new(test_dir, framework.clone())?
        .with_init_config(Some(Arc::new(move |_, config, _| {
            *config = node_config.clone();
        })))
        .with_init_genesis_config(Some(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = EPOCH_LENGTH_SECS;
            genesis_config.recurring_lockup_duration_secs = 7200;

            match env::var("ENABLE_KEYLESS_DEFAULT") {
                Ok(val) if val.as_str() == "1" => {
                    let response = ureq::get("https://api.devnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey").call();
                    let json: Value = response.into_json().expect("Failed to parse JSON");
                    configure_keyless_with_vk(genesis_config, json).unwrap();
                },
                _ => {},
            };

            if let Ok(url) = env::var("INSTALL_KEYLESS_GROTH16_VK_FROM_URL") {
                let response = ureq::get(&url).call();
                let json: Value = response.into_json().expect("Failed to parse JSON");
                configure_keyless_with_vk(genesis_config, json).unwrap();
            };

            if let Ok(path) = env::var("INSTALL_KEYLESS_GROTH16_VK_FROM_PATH") {
                let file_content = fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read verification key file: {}", path));
                let json: Value = serde_json::from_str(&file_content).expect("Failed to parse JSON");
                configure_keyless_with_vk(genesis_config, json).unwrap();
            };
        })))
        .with_randomize_first_validator_ports(random_ports);
    let (root_key, _genesis, genesis_waypoint, mut validators) = builder.build(rng)?;

    // Write the mint key to disk
    let serialized_keys = bcs::to_bytes(&root_key)?;
    let mut key_file = fs::File::create(aptos_root_key_path)?;
    key_file.write_all(&serialized_keys)?;

    // Build a waypoint file so that clients / docker can grab it easily
    let waypoint_file_path = test_dir.join("waypoint.txt");
    Write::write_all(
        &mut fs::File::create(waypoint_file_path)?,
        genesis_waypoint.to_string().as_bytes(),
    )?;

    aptos_config::config::sanitize_node_config(validators[0].config.override_config_mut())?;

    let mut node_config = validators[0].config.override_config().clone();

    // Enable the AdminService.
    node_config.admin_service.enabled = Some(true);

    Ok(node_config)
}

fn configure_keyless_with_vk(
    genesis_config: &mut GenesisConfiguration,
    json: Value,
) -> anyhow::Result<()> {
    let vk = parse_groth16_vk_from_json(&json)?;
    genesis_config.keyless_groth16_vk = Some(vk);
    genesis_config.jwk_consensus_config_override =
        Some(OnChainJWKConsensusConfig::default_enabled());
    Ok(())
}

fn parse_groth16_vk_from_json(json: &Value) -> Result<Groth16VerificationKey, anyhow::Error> {
    println!(
        "Loading verification key from JSON:\n{}",
        serde_json::to_string_pretty(&json).unwrap()
    );
    let vk_data = json["data"].clone();
    let gamma_abc_g1 = vk_data["gamma_abc_g1"]
        .as_array()
        .unwrap()
        .iter()
        .map(|val| {
            let hex_str = val.as_str().unwrap();
            let cleaned_hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            hex::decode(cleaned_hex).expect("Failed to decode gamma_abc_g1 hex")
        })
        .collect::<Vec<Vec<u8>>>();

    Ok(Groth16VerificationKey {
        alpha_g1: decode_hex_field(&vk_data, "alpha_g1").unwrap(),
        beta_g2: decode_hex_field(&vk_data, "beta_g2").unwrap(),
        gamma_g2: decode_hex_field(&vk_data, "gamma_g2").unwrap(),
        delta_g2: decode_hex_field(&vk_data, "delta_g2").unwrap(),
        gamma_abc_g1,
    })
}

fn decode_hex_field(json: &Value, field: &str) -> Result<Vec<u8>, anyhow::Error> {
    let hex_str = json[field]
        .as_str()
        .ok_or_else(|| anyhow!("Missing or invalid {} in verification key file", field))?;

    // Strip "0x" prefix if present
    let cleaned_hex = hex_str.strip_prefix("0x").unwrap_or(hex_str);

    // Decode hex string
    hex::decode(cleaned_hex).with_context(|| format!("Failed to decode hex for field {}", field))
}

/// Initializes the node environment and starts the node
pub fn setup_environment_and_start_node(
    mut node_config: NodeConfig,
    remote_log_rx: Option<mpsc::Receiver<TelemetryLog>>,
    logger_filter_update_job: Option<LoggerFilterUpdater>,
    api_port_tx: Option<oneshot::Sender<u16>>,
    indexer_grpc_port_tx: Option<oneshot::Sender<u16>>,
) -> anyhow::Result<AptosHandle> {
    // Log the node config at node startup
    node_config.log_all_configs();

    // Starts the admin service
    let mut admin_service = services::start_admin_service(&node_config);

    // Set up the storage database and any RocksDB checkpoints
    let (db_rw, backup_service, genesis_waypoint, indexer_db_opt, update_receiver) =
        storage::initialize_database_and_checkpoints(&mut node_config)?;

    admin_service.set_aptos_db(db_rw.clone().into());

    // Set the Aptos VM configurations
    utils::set_aptos_vm_configurations(&node_config);

    // Obtain the chain_id from the DB
    let chain_id = utils::fetch_chain_id(&db_rw)?;

    // Set the chain_id in global AptosNodeIdentity
    aptos_node_identity::set_chain_id(chain_id)?;

    // Start the telemetry service (as early as possible and before any blocking calls)
    let telemetry_runtime = services::start_telemetry_service(
        &node_config,
        remote_log_rx,
        logger_filter_update_job,
        chain_id,
    );

    // Create an event subscription service (and reconfig subscriptions for consensus and mempool)
    let (
        mut event_subscription_service,
        mempool_reconfig_subscription,
        consensus_observer_reconfig_subscription,
        consensus_reconfig_subscription,
        dkg_subscriptions,
        jwk_consensus_subscriptions,
    ) = state_sync::create_event_subscription_service(&node_config, &db_rw);

    // Set up the networks and gather the application network handles
    let peers_and_metadata = network::create_peers_and_metadata(&node_config);
    let (
        network_runtimes,
        consensus_network_interfaces,
        consensus_observer_network_interfaces,
        dkg_network_interfaces,
        jwk_consensus_network_interfaces,
        mempool_network_interfaces,
        peer_monitoring_service_network_interfaces,
        storage_service_network_interfaces,
    ) = network::setup_networks_and_get_interfaces(
        &node_config,
        chain_id,
        peers_and_metadata.clone(),
        &mut event_subscription_service,
    );

    // Start the peer monitoring service
    let peer_monitoring_service_runtime = services::start_peer_monitoring_service(
        &node_config,
        peer_monitoring_service_network_interfaces,
        db_rw.reader.clone(),
    );

    // Start state sync and get the notification endpoints for mempool and consensus
    let (aptos_data_client, state_sync_runtimes, mempool_listener, consensus_notifier) =
        state_sync::start_state_sync_and_get_notification_handles(
            &node_config,
            storage_service_network_interfaces,
            genesis_waypoint,
            event_subscription_service,
            db_rw.clone(),
        )?;

    // Start the node inspection service
    services::start_node_inspection_service(
        &node_config,
        aptos_data_client,
        peers_and_metadata.clone(),
    );

    // Bootstrap the API and indexer
    let (
        mempool_client_receiver,
        api_runtime,
        indexer_table_info_runtime,
        indexer_runtime,
        indexer_grpc_runtime,
        internal_indexer_db_runtime,
        mempool_client_sender,
    ) = services::bootstrap_api_and_indexer(
        &node_config,
        db_rw.clone(),
        chain_id,
        indexer_db_opt,
        update_receiver,
        api_port_tx,
        indexer_grpc_port_tx,
    )?;

    // Set mempool client sender in order to enable the Mempool API in the admin service
    admin_service.set_mempool_client_sender(mempool_client_sender);

    // Create mempool and get the consensus to mempool sender
    let (mempool_runtime, consensus_to_mempool_sender) =
        services::start_mempool_runtime_and_get_consensus_sender(
            &mut node_config,
            &db_rw,
            mempool_reconfig_subscription,
            mempool_network_interfaces,
            mempool_listener,
            mempool_client_receiver,
            peers_and_metadata,
        );

    // Create the DKG runtime and get the VTxn pool
    let (vtxn_pool, dkg_runtime) =
        consensus::create_dkg_runtime(&mut node_config, dkg_subscriptions, dkg_network_interfaces);

    // Create the JWK consensus runtime
    let jwk_consensus_runtime = consensus::create_jwk_consensus_runtime(
        &mut node_config,
        jwk_consensus_subscriptions,
        jwk_consensus_network_interfaces,
        &vtxn_pool,
    );

    // Wait until state sync has been initialized
    debug!("Waiting until state sync is initialized!");
    state_sync_runtimes.block_until_initialized();
    debug!("State sync initialization complete.");

    // Create the consensus observer and publisher (if enabled)
    let (consensus_observer_runtime, consensus_publisher_runtime, consensus_publisher) =
        consensus::create_consensus_observer_and_publisher(
            &node_config,
            consensus_observer_network_interfaces,
            consensus_notifier.clone(),
            consensus_to_mempool_sender.clone(),
            db_rw.clone(),
            consensus_observer_reconfig_subscription,
        );

    // Create the consensus runtime (if enabled)
    let consensus_runtime = consensus::create_consensus_runtime(
        &node_config,
        db_rw.clone(),
        consensus_reconfig_subscription,
        consensus_network_interfaces,
        consensus_notifier.clone(),
        consensus_to_mempool_sender.clone(),
        vtxn_pool,
        consensus_publisher.clone(),
        &mut admin_service,
    );

    Ok(AptosHandle {
        _admin_service: admin_service,
        _api_runtime: api_runtime,
        _backup_runtime: backup_service,
        _consensus_observer_runtime: consensus_observer_runtime,
        _consensus_publisher_runtime: consensus_publisher_runtime,
        _consensus_runtime: consensus_runtime,
        _dkg_runtime: dkg_runtime,
        _indexer_grpc_runtime: indexer_grpc_runtime,
        _indexer_runtime: indexer_runtime,
        _indexer_table_info_runtime: indexer_table_info_runtime,
        _jwk_consensus_runtime: jwk_consensus_runtime,
        _mempool_runtime: mempool_runtime,
        _network_runtimes: network_runtimes,
        _peer_monitoring_service_runtime: peer_monitoring_service_runtime,
        _state_sync_runtimes: state_sync_runtimes,
        _telemetry_runtime: telemetry_runtime,
        _indexer_db_runtime: internal_indexer_db_runtime,
    })
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    AptosNodeArgs::command().debug_assert()
}
