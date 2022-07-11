// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::anyhow;
use aptos_api::runtime::bootstrap as bootstrap_api;
use aptos_config::{
    config::{
        AptosDataClientConfig, BaseConfig, DataStreamingServiceConfig, NetworkConfig, NodeConfig,
        PersistableConfig, StorageServiceConfig,
    },
    network_id::NetworkId,
    utils::get_genesis_txn,
};
use aptos_data_client::aptosnet::AptosNetDataClient;
use aptos_infallible::RwLock;
use aptos_logger::{prelude::*, Level};
use aptos_sf_stream::runtime::bootstrap as bootstrap_sf_stream;
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_time_service::TimeService;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS, account_view::AccountView, chain_id::ChainId,
    on_chain_config::ON_CHAIN_CONFIG_REGISTRY, waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use backup_service::start_backup_service;
use clap::Parser;
use consensus::consensus_provider::start_consensus;
use consensus_notifications::ConsensusNotificationListener;
use data_streaming_service::{
    streaming_client::{new_streaming_service_client_listener_pair, StreamingServiceClient},
    streaming_service::DataStreamingService,
};
use event_notifications::EventSubscriptionService;
use executor::{chunk_executor::ChunkExecutor, db_bootstrapper::maybe_bootstrap};
use futures::channel::mpsc::channel;
use hex::FromHex;
use mempool_notifications::MempoolNotificationSender;
use network::application::storage::PeerMetadataStorage;
use network_builder::builder::NetworkBuilder;
use rand::{rngs::StdRng, SeedableRng};
use state_sync_driver::driver_factory::DriverFactory;
use state_sync_driver::driver_factory::StateSyncRuntimes;
use state_sync_driver::metadata_storage::PersistentMetadataStorage;
use std::{
    boxed::Box,
    collections::{HashMap, HashSet},
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Instant,
};
use storage_interface::{state_view::LatestDbStateCheckpointView, DbReaderWriter};
use storage_service_client::{StorageServiceClient, StorageServiceMultiSender};
use storage_service_server::{
    network::StorageServiceNetworkEvents, StorageReader, StorageServiceServer,
};
use tokio::runtime::{Builder, Runtime};

const AC_SMP_CHANNEL_BUFFER_SIZE: usize = 1_024;
const INTRA_NODE_CHANNEL_BUFFER_SIZE: usize = 1;
const MEMPOOL_NETWORK_CHANNEL_BUFFER_SIZE: usize = 1_024;

/// Runs an aptos fullnode or validator
#[derive(Clone, Debug, Parser)]
#[clap(name = "Aptos Node", author, version)]
pub struct AptosNodeArgs {
    /// Path to node configuration file (or template for local test mode)
    #[clap(short = 'f', long, parse(from_os_str), required_unless = "test")]
    config: Option<PathBuf>,

    /// Directory to run test mode out of
    ///
    /// Repeated runs will start up from previous state
    #[clap(long, parse(from_os_str), requires("test"))]
    test_dir: Option<PathBuf>,

    /// Enable to run a single validator node testnet
    #[clap(long)]
    test: bool,

    /// Random number generator seed to use when starting a single validator testnet
    #[clap(long, parse(try_from_str = FromHex::from_hex), requires("test"))]
    seed: Option<[u8; 32]>,

    /// Enable using random ports instead of ports from the node configuration
    #[clap(long, requires("test"))]
    random_ports: bool,

    /// Paths to Aptos framework module blobs to be included in genesis.
    ///
    /// Can be both files and directories
    #[clap(long, requires("test"))]
    genesis_modules: Option<Vec<PathBuf>>,

    /// Enable lazy mode
    ///
    /// Setting this flag will set `consensus#mempool_poll_count` config to `u64::MAX` and
    /// only commit a block when there is user transaction in mempool.
    #[clap(long, requires("test"))]
    lazy: bool,
}

impl AptosNodeArgs {
    pub fn run(self) {
        if self.test {
            println!("Entering test mode, this should never be used in production!");
            let rng = self
                .seed
                .map(StdRng::from_seed)
                .unwrap_or_else(StdRng::from_entropy);
            let genesis_modules = if let Some(module_paths) = self.genesis_modules {
                framework::load_modules_from_paths(&module_paths)
            } else {
                cached_framework_packages::module_blobs().to_vec()
            };
            load_test_environment(
                self.config,
                self.test_dir,
                self.random_ports,
                self.lazy,
                genesis_modules,
                rng,
            )
            .expect("Test mode should start correctly");
        } else {
            // Load the config file
            let config_path = self.config.expect("Config is required to launch node");
            let config = NodeConfig::load(config_path.clone()).unwrap_or_else(|error| {
                panic!(
                    "Failed to load node config file! Given file path: {:?}. Error: {:?}",
                    config_path, error
                )
            });
            println!("Using node config {:?}", &config);

            // Start the node
            start(config, None).expect("Node should start correctly");
        };
    }
}

/// Runtime handle to ensure that all inner runtimes stay in scope
pub struct AptosHandle {
    _api: Runtime,
    _backup: Runtime,
    _consensus_runtime: Option<Runtime>,
    _mempool: Runtime,
    _network_runtimes: Vec<Runtime>,
    _sf_stream: Option<Runtime>,
    _state_sync_runtimes: StateSyncRuntimes,
    _telemetry_runtime: Option<Runtime>,
}

/// Start an aptos node
pub fn start(config: NodeConfig, log_file: Option<PathBuf>) -> anyhow::Result<()> {
    crash_handler::setup_panic_handler();

    let mut logger = aptos_logger::Logger::new();
    logger
        .channel_size(config.logger.chan_size)
        .is_async(config.logger.is_async)
        .level(config.logger.level)
        .console_port(config.logger.console_port)
        .read_env();
    if config.logger.enable_backtrace {
        logger.enable_backtrace();
    }
    if let Some(log_file) = log_file {
        logger.printer(Box::new(FileWriter::new(log_file)));
    }
    let _logger = Some(logger.build());

    // Let's now log some important information, since the logger is set up
    info!(config = config, "Loaded AptosNode config");

    if fail::has_failpoints() {
        warn!("Failpoints is enabled");
        if let Some(failpoints) = &config.failpoints {
            for (point, actions) in failpoints {
                fail::cfg(point, actions).expect("fail to set actions for failpoint");
            }
        }
    } else if config.failpoints.is_some() {
        warn!("failpoints is set in config, but the binary doesn't compile with this feature");
    }

    let _node_handle = setup_environment(config)?;
    let term = Arc::new(AtomicBool::new(false));

    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
    Ok(())
}

const EPOCH_LENGTH_SECS: u64 = 60;

pub fn load_test_environment<R>(
    config_path: Option<PathBuf>,
    test_dir: Option<PathBuf>,
    random_ports: bool,
    lazy: bool,
    genesis_modules: Vec<Vec<u8>>,
    rng: R,
) -> anyhow::Result<()>
where
    R: ::rand::RngCore + ::rand::CryptoRng,
{
    // If there wasn't a testnet directory given, create a temp one
    let test_dir = if let Some(test_dir) = test_dir {
        test_dir
    } else {
        aptos_temppath::TempPath::new().as_ref().to_path_buf()
    };

    // Create the directories for the node
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&test_dir)?;
    let test_dir = test_dir.canonicalize()?;

    // The validator builder puts the first node in the 0 directory
    let validator_config_path = test_dir.join("0").join("node.yaml");
    let aptos_root_key_path = test_dir.join("mint.key");

    // If there's already a config, use it
    let config = if validator_config_path.exists() {
        NodeConfig::load(&validator_config_path)
            .map_err(|err| anyhow!("Unable to load config: {}", err))?
    } else {
        // Build a single validator network with a generated config
        let mut template = NodeConfig::default_for_validator();

        // If a config path was provided, use that as the template
        if let Some(config_path) = config_path {
            if let Ok(config) = NodeConfig::load_config(config_path) {
                template = config;
            }
        }

        template.logger.level = Level::Debug;
        // enable REST and JSON-RPC API
        template.api.address = format!("0.0.0.0:{}", template.api.address.port()).parse()?;
        if lazy {
            template.consensus.quorum_store_poll_count = u64::MAX;
        }

        // Build genesis and validator node
        let builder = aptos_genesis::builder::Builder::new(&test_dir, genesis_modules)?
            .with_init_config(Some(Arc::new(move |_, config, _| {
                *config = template.clone();
            })))
            .with_init_genesis_config(Some(Arc::new(|genesis_config| {
                genesis_config.allow_new_validators = true;
                genesis_config.epoch_duration_secs = EPOCH_LENGTH_SECS;
                genesis_config.recurring_lockup_duration_secs = 86400;
            })))
            .with_randomize_first_validator_ports(random_ports);

        let (root_key, _genesis, genesis_waypoint, validators) = builder.build(rng)?;

        // Write the mint key to disk
        let serialized_keys = bcs::to_bytes(&root_key)?;
        let mut key_file = std::fs::File::create(&aptos_root_key_path)?;
        key_file.write_all(&serialized_keys)?;

        // Build a waypoint file so that clients / docker can grab it easily
        let waypoint_file_path = test_dir.join("waypoint.txt");
        std::io::Write::write_all(
            &mut std::fs::File::create(&waypoint_file_path)?,
            genesis_waypoint.to_string().as_bytes(),
        )?;

        validators[0].config.clone()
    };

    // Prepare log file since we cannot automatically route logs to stderr
    let log_file = test_dir.join("validator.log");

    println!("Completed generating configuration:");
    println!("\tLog file: {:?}", log_file);
    println!("\tTest dir: {:?}", test_dir);
    println!("\tAptos root key path: {:?}", aptos_root_key_path);
    println!("\tWaypoint: {}", config.base.waypoint.genesis_waypoint());
    println!("\tChainId: {}", ChainId::test());
    println!("\tREST API endpoint: {}", &config.api.address);
    println!(
        "\tFullNode network: {}",
        &config.full_node_networks[0].listen_address
    );
    if lazy {
        println!("\tLazy mode is enabled");
    }

    println!("\nAptos is running, press ctrl-c to exit\n");

    start(config, Some(log_file))
}

// Fetch chain ID from on-chain resource
fn fetch_chain_id(db: &DbReaderWriter) -> anyhow::Result<ChainId> {
    let db_state_view = db
        .reader
        .latest_state_checkpoint_view()
        .map_err(|err| anyhow!("[aptos-node] failed to create db state view {}", err))?;
    Ok(db_state_view
        .as_account_with_state_view(&CORE_CODE_ADDRESS)
        .get_chain_id_resource()
        .map_err(|err| anyhow!("[aptos-node] failed to get chain id resource {}", err))?
        .expect("[aptos-node] missing chain ID resource")
        .chain_id())
}

fn create_state_sync_runtimes<M: MempoolNotificationSender + 'static>(
    node_config: &NodeConfig,
    storage_service_server_network_handles: Vec<StorageServiceNetworkEvents>,
    storage_service_client_network_handles: HashMap<
        NetworkId,
        storage_service_client::StorageServiceNetworkSender,
    >,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    mempool_notifier: M,
    consensus_listener: ConsensusNotificationListener,
    waypoint: Waypoint,
    event_subscription_service: EventSubscriptionService,
    db_rw: DbReaderWriter,
) -> anyhow::Result<StateSyncRuntimes> {
    // Start the state sync storage service
    let storage_service_runtime = setup_state_sync_storage_service(
        node_config.state_sync.storage_service,
        storage_service_server_network_handles,
        &db_rw,
    )?;

    // Start the data client
    let (aptos_data_client, aptos_data_client_runtime) = setup_aptos_data_client(
        node_config.state_sync.storage_service,
        node_config.state_sync.aptos_data_client,
        node_config.base.clone(),
        storage_service_client_network_handles,
        peer_metadata_storage,
    )?;

    // Start the data streaming service
    let (streaming_service_client, streaming_service_runtime) = setup_data_streaming_service(
        node_config.state_sync.data_streaming_service,
        aptos_data_client.clone(),
    )?;

    // Create the chunk executor and persistent storage
    let chunk_executor = Arc::new(ChunkExecutor::<AptosVM>::new(db_rw.clone()));
    let metadata_storage = PersistentMetadataStorage::new(&node_config.storage.dir());

    // Create the state sync driver factory
    let state_sync = DriverFactory::create_and_spawn_driver(
        true,
        node_config,
        waypoint,
        db_rw,
        chunk_executor,
        mempool_notifier,
        metadata_storage,
        consensus_listener,
        event_subscription_service,
        aptos_data_client,
        streaming_service_client,
    );

    // Create and return the new state sync handle
    Ok(StateSyncRuntimes::new(
        aptos_data_client_runtime,
        state_sync,
        storage_service_runtime,
        streaming_service_runtime,
    ))
}

fn setup_data_streaming_service(
    config: DataStreamingServiceConfig,
    aptos_data_client: AptosNetDataClient,
) -> anyhow::Result<(StreamingServiceClient, Runtime)> {
    // Create the data streaming service
    let (streaming_service_client, streaming_service_listener) =
        new_streaming_service_client_listener_pair();
    let data_streaming_service =
        DataStreamingService::new(config, aptos_data_client, streaming_service_listener);

    // Start the data streaming service
    let streaming_service_runtime = Builder::new_multi_thread()
        .thread_name("data-streaming-service")
        .enable_all()
        .build()
        .map_err(|err| anyhow!("Failed to create data streaming service {}", err))?;
    streaming_service_runtime.spawn(data_streaming_service.start_service());

    Ok((streaming_service_client, streaming_service_runtime))
}

fn setup_aptos_data_client(
    storage_service_config: StorageServiceConfig,
    aptos_data_client_config: AptosDataClientConfig,
    base_config: BaseConfig,
    network_handles: HashMap<NetworkId, storage_service_client::StorageServiceNetworkSender>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> anyhow::Result<(AptosNetDataClient, Runtime)> {
    // Combine all storage service client handles
    let network_client = StorageServiceClient::new(
        StorageServiceMultiSender::new(network_handles),
        peer_metadata_storage,
    );

    // Create a new runtime for the data client
    let aptos_data_client_runtime = Builder::new_multi_thread()
        .thread_name("aptos-data-client")
        .enable_all()
        .build()
        .map_err(|err| anyhow!("Failed to create aptos data client {}", err))?;

    // Create the data client and spawn the data poller
    let (aptos_data_client, data_summary_poller) = AptosNetDataClient::new(
        aptos_data_client_config,
        base_config,
        storage_service_config,
        TimeService::real(),
        network_client,
        Some(aptos_data_client_runtime.handle().clone()),
    );
    aptos_data_client_runtime.spawn(data_summary_poller.start_poller());

    Ok((aptos_data_client, aptos_data_client_runtime))
}

fn setup_state_sync_storage_service(
    config: StorageServiceConfig,
    network_handles: Vec<StorageServiceNetworkEvents>,
    db_rw: &DbReaderWriter,
) -> anyhow::Result<Runtime> {
    // Create a new state sync storage service runtime
    let storage_service_runtime = Builder::new_multi_thread()
        .thread_name("storage-service-server")
        .enable_all()
        .build()
        .map_err(|err| anyhow!("Failed to start state sync storage service {}", err))?;

    // Spawn all state sync storage service servers on the same runtime
    let storage_reader = StorageReader::new(config, Arc::clone(&db_rw.reader));
    for events in network_handles {
        let service = StorageServiceServer::new(
            config,
            storage_service_runtime.handle().clone(),
            storage_reader.clone(),
            TimeService::real(),
            events,
        );
        storage_service_runtime.spawn(service.start());
    }

    Ok(storage_service_runtime)
}

pub fn setup_environment(node_config: NodeConfig) -> anyhow::Result<AptosHandle> {
    // Start the node inspection service
    let node_config_clone = node_config.clone();
    thread::spawn(move || {
        inspection_service::inspection_service::start_inspection_service(node_config_clone)
    });

    // Open the database
    let mut instant = Instant::now();
    let (aptos_db, db_rw) = DbReaderWriter::wrap(
        AptosDB::open(
            &node_config.storage.dir(),
            false, /* readonly */
            node_config.storage.storage_pruner_config,
            node_config.storage.rocksdb_configs,
            node_config.storage.enable_indexer,
            node_config.storage.target_snapshot_size,
        )
        .map_err(|err| anyhow!("DB failed to open {}", err))?,
    );
    let backup_service = start_backup_service(
        node_config.storage.backup_service_address,
        Arc::clone(&aptos_db),
    );

    let genesis_waypoint = node_config.base.waypoint.genesis_waypoint();
    // if there's genesis txn and waypoint, commit it if the result matches.
    if let Some(genesis) = get_genesis_txn(&node_config) {
        maybe_bootstrap::<AptosVM>(&db_rw, genesis, genesis_waypoint)
            .map_err(|err| anyhow!("DB failed to bootstrap {}", err))?;
    } else {
        info!("Genesis txn not provided, it's fine if you don't expect to apply it otherwise please double check config");
    }
    AptosVM::set_concurrency_level_once(node_config.execution.concurrency_level as usize);
    AptosVM::set_num_proof_reading_threads_once(
        node_config.execution.num_proof_reading_threads as usize,
    );

    debug!(
        "Storage service started in {} ms",
        instant.elapsed().as_millis()
    );

    let chain_id = fetch_chain_id(&db_rw)?;
    let mut network_runtimes = vec![];
    let mut mempool_network_handles = vec![];
    let mut consensus_network_handles = None;
    let mut storage_service_server_network_handles = vec![];
    let mut storage_service_client_network_handles = HashMap::new();

    // Create an event subscription service so that components can be notified of events and reconfigs
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(db_rw.clone())),
    );
    let mempool_reconfig_subscription =
        event_subscription_service.subscribe_to_reconfigurations()?;

    // Create a consensus subscription for reconfiguration events (if this node is a validator).
    let consensus_reconfig_subscription = if node_config.base.role.is_validator() {
        Some(event_subscription_service.subscribe_to_reconfigurations()?)
    } else {
        None
    };

    // Gather all network configs into a single vector.
    let mut network_configs: Vec<&NetworkConfig> = node_config.full_node_networks.iter().collect();
    if let Some(network_config) = node_config.validator_network.as_ref() {
        network_configs.push(network_config);
    }

    // Instantiate every network and collect the requisite endpoints for state_sync, mempool, and consensus.
    let mut network_ids = HashSet::new();
    network_configs.iter().for_each(|config| {
        let network_id = config.network_id;
        // Guarantee there is only one of this network
        if network_ids.contains(&network_id) {
            panic!(
                "Duplicate NetworkId: '{}'.  Can't start node with duplicate networks",
                network_id
            );
        }
        network_ids.insert(network_id);
    });
    let network_ids: Vec<_> = network_ids.into_iter().collect();

    let peer_metadata_storage = PeerMetadataStorage::new(&network_ids);
    for network_config in network_configs.into_iter() {
        debug!("Creating runtime for {}", network_config.network_id);
        let mut runtime_builder = Builder::new_multi_thread();
        runtime_builder
            .thread_name(format!("network-{}", network_config.network_id))
            .enable_all();
        if let Some(runtime_threads) = network_config.runtime_threads {
            runtime_builder.worker_threads(runtime_threads);
        }
        let runtime = runtime_builder.build().map_err(|err| {
            anyhow!(
                "Failed to start runtime.  Won't be able to start networking. {}",
                err
            )
        })?;

        // Entering here gives us a runtime to instantiate all the pieces of the builder
        let _enter = runtime.enter();

        // Perform common instantiation steps
        let mut network_builder = NetworkBuilder::create(
            chain_id,
            node_config.base.role,
            network_config,
            TimeService::real(),
            Some(&mut event_subscription_service),
            peer_metadata_storage.clone(),
        );
        let network_id = network_config.network_id;

        // TODO(philiphayes): configure which networks we serve the storage service
        // on? for example, if we're a light node we wouldn't want to provide the
        // storage service at all.

        // Register the network-facing storage service with Network.
        let storage_service_events =
            network_builder.add_service(&storage_service_server::network::network_endpoint_config(
                node_config.state_sync.storage_service,
            ));
        storage_service_server_network_handles.push(storage_service_events);

        // Register the storage-service clients with Network
        let storage_service_sender =
            network_builder.add_client(&storage_service_client::network_endpoint_config());
        storage_service_client_network_handles.insert(network_id, storage_service_sender);

        // Create the endpoints to connect the Network to mempool.
        let (mempool_sender, mempool_events) = network_builder.add_p2p_service(
            &aptos_mempool::network::network_endpoint_config(MEMPOOL_NETWORK_CHANNEL_BUFFER_SIZE),
        );
        mempool_network_handles.push((network_id, mempool_sender, mempool_events));

        // Perform steps relevant specifically to Validator networks.
        if network_id.is_validator_network() {
            // A valid config is allowed to have at most one ValidatorNetwork
            // TODO:  `expect_none` would be perfect here, once it is stable.
            if consensus_network_handles.is_some() {
                panic!("There can be at most one validator network!");
            }

            consensus_network_handles = Some(
                network_builder
                    .add_p2p_service(&consensus::network_interface::network_endpoint_config()),
            );
        }

        let network_context = network_builder.network_context();
        network_builder.build(runtime.handle().clone());
        network_builder.start();
        debug!("Network built for network context: {}", network_context);
        network_runtimes.push(runtime);
    }

    // TODO set up on-chain discovery network based on UpstreamConfig.fallback_network
    // and pass network handles to mempool/state sync

    // For state sync to send notifications to mempool and receive notifications from consensus.
    let (mempool_notifier, mempool_listener) =
        mempool_notifications::new_mempool_notifier_listener_pair();
    let (consensus_notifier, consensus_listener) =
        consensus_notifications::new_consensus_notifier_listener_pair(
            node_config
                .state_sync
                .state_sync_driver
                .commit_notification_timeout_ms,
        );

    // Create the state sync runtimes
    let state_sync_runtimes = create_state_sync_runtimes(
        &node_config,
        storage_service_server_network_handles,
        storage_service_client_network_handles,
        peer_metadata_storage.clone(),
        mempool_notifier,
        consensus_listener,
        genesis_waypoint,
        event_subscription_service,
        db_rw.clone(),
    )?;

    let (mp_client_sender, mp_client_events) = channel(AC_SMP_CHANNEL_BUFFER_SIZE);

    let api_runtime = bootstrap_api(
        &node_config,
        chain_id,
        aptos_db.clone(),
        mp_client_sender.clone(),
    )?;
    let sf_runtime = match bootstrap_sf_stream(&node_config, chain_id, aptos_db, mp_client_sender) {
        None => None,
        Some(res) => Some(res?),
    };

    let mut consensus_runtime = None;
    let (consensus_to_mempool_sender, consensus_to_mempool_receiver) =
        channel(INTRA_NODE_CHANNEL_BUFFER_SIZE);

    instant = Instant::now();
    let mempool = aptos_mempool::bootstrap(
        &node_config,
        Arc::clone(&db_rw.reader),
        mempool_network_handles,
        mp_client_events,
        consensus_to_mempool_receiver,
        mempool_listener,
        mempool_reconfig_subscription,
        peer_metadata_storage.clone(),
    );
    debug!("Mempool started in {} ms", instant.elapsed().as_millis());

    assert!(
        !node_config.consensus.use_quorum_store,
        "QuorumStore is not yet implemented"
    );
    assert_ne!(
        node_config.consensus.use_quorum_store,
        node_config.mempool.shared_mempool_validator_broadcast,
        "Shared mempool validator broadcast must be turned off when QuorumStore is on, and vice versa"
    );

    // StateSync should be instantiated and started before Consensus to avoid a cyclic dependency:
    // network provider -> consensus -> state synchronizer -> network provider.  This has resulted
    // in a deadlock as observed in GitHub issue #749.
    if let Some((consensus_network_sender, consensus_network_events)) = consensus_network_handles {
        // Make sure that state synchronizer is caught up at least to its waypoint
        // (in case it's present). There is no sense to start consensus prior to that.
        // TODO: Note that we need the networking layer to be able to discover & connect to the
        // peers with potentially outdated network identity public keys.
        debug!("Wait until state sync is initialized");
        state_sync_runtimes.block_until_initialized();
        debug!("State sync initialization complete.");

        // Initialize and start consensus.
        instant = Instant::now();
        consensus_runtime = Some(start_consensus(
            &node_config,
            consensus_network_sender,
            consensus_network_events,
            Arc::new(consensus_notifier),
            consensus_to_mempool_sender,
            db_rw,
            consensus_reconfig_subscription
                .expect("Consensus requires a reconfiguration subscription!"),
            peer_metadata_storage,
        ));
        debug!("Consensus started in {} ms", instant.elapsed().as_millis());
    }

    // Create the telemetry service
    let telemetry_runtime = aptos_telemetry::service::start_telemetry_service(
        node_config.clone(),
        chain_id.to_string(),
    );

    Ok(AptosHandle {
        _api: api_runtime,
        _backup: backup_service,
        _consensus_runtime: consensus_runtime,
        _mempool: mempool,
        _network_runtimes: network_runtimes,
        _sf_stream: sf_runtime,
        _state_sync_runtimes: state_sync_runtimes,
        _telemetry_runtime: telemetry_runtime,
    })
}
