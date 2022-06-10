// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

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
use aptos_logger::{prelude::*, Logger};
use aptos_metrics::{metric_server, system_information};
use aptos_state_view::account_with_state_view::AsAccountWithStateView;
use aptos_telemetry::{
    constants::{
        APTOS_NODE_BUILD_INFORMATION, APTOS_NODE_PUSH_METRICS, APTOS_NODE_SYSTEM_INFORMATION,
        CHAIN_ID_METRIC, NODE_PUSH_METRICS_FREQ_SECS, NODE_SYS_INFO_FREQ_SECS, PEER_ID_METRIC,
        SYNCED_VERSION_METRIC,
    },
    send_env_data,
};
use aptos_time_service::TimeService;
use aptos_types::{
    account_config::aptos_root_address, account_view::AccountView, chain_id::ChainId,
    move_resource::MoveStorage, on_chain_config::ON_CHAIN_CONFIG_REGISTRY, waypoint::Waypoint,
};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use backup_service::start_backup_service;
use consensus::consensus_provider::start_consensus;
use consensus_notifications::ConsensusNotificationListener;
use data_streaming_service::{
    streaming_client::{new_streaming_service_client_listener_pair, StreamingServiceClient},
    streaming_service::DataStreamingService,
};
use debug_interface::node_debug_service::NodeDebugService;
use event_notifications::EventSubscriptionService;
use executor::{chunk_executor::ChunkExecutor, db_bootstrapper::maybe_bootstrap};
use futures::{channel::mpsc::channel, stream::StreamExt};
use mempool_notifications::MempoolNotificationSender;
use network::application::storage::PeerMetadataStorage;
use network_builder::builder::NetworkBuilder;
use state_sync_multiplexer::{
    state_sync_v1_network_config, StateSyncMultiplexer, StateSyncRuntimes,
};
use state_sync_v1::network::{StateSyncEvents, StateSyncSender};
use std::{
    boxed::Box,
    collections::{BTreeMap, HashMap, HashSet},
    io::Write,
    net::ToSocketAddrs,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Instant,
};
use storage_interface::{state_view::LatestDbStateCheckpointView, DbReaderWriter};
use storage_service::start_storage_service_with_db;
use storage_service_client::{StorageServiceClient, StorageServiceMultiSender};
use storage_service_server::{
    network::StorageServiceNetworkEvents, StorageReader, StorageServiceServer,
};
use tokio::runtime::{Builder, Runtime};
use tokio_stream::wrappers::IntervalStream;

const AC_SMP_CHANNEL_BUFFER_SIZE: usize = 1_024;
const INTRA_NODE_CHANNEL_BUFFER_SIZE: usize = 1;
const MEMPOOL_NETWORK_CHANNEL_BUFFER_SIZE: usize = 1_024;

pub struct AptosHandle {
    _api: Runtime,
    _backup: Runtime,
    _consensus_runtime: Option<Runtime>,
    _debug: NodeDebugService,
    _mempool: Runtime,
    _network_runtimes: Vec<Runtime>,
    _state_sync_runtimes: StateSyncRuntimes,
    _telemetry_runtime: Runtime,
}

pub fn start(config: &NodeConfig, log_file: Option<PathBuf>) {
    crash_handler::setup_panic_handler();

    let mut logger = aptos_logger::Logger::new();
    logger
        .channel_size(config.logger.chan_size)
        .is_async(config.logger.is_async)
        .level(config.logger.level)
        .read_env();
    if config.logger.enable_backtrace {
        logger.enable_backtrace();
    }
    if let Some(log_file) = log_file {
        logger.printer(Box::new(FileWriter::new(log_file)));
    }
    let logger = Some(logger.build());

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

    let _node_handle = setup_environment(config, logger);
    let term = Arc::new(AtomicBool::new(false));

    while !term.load(Ordering::Acquire) {
        std::thread::park();
    }
}

pub fn load_test_environment<R>(
    config_path: Option<PathBuf>,
    random_ports: bool,
    lazy: bool,
    genesis_modules: Vec<Vec<u8>>,
    rng: R,
) where
    R: ::rand::RngCore + ::rand::CryptoRng,
{
    let config_temp_path = aptos_temppath::TempPath::new();

    let (try_load, config_path) = if let Some(config_path) = config_path {
        (
            config_path.join("0").join("node.yaml").exists(),
            config_path,
        )
    } else {
        (false, config_temp_path.as_ref().to_path_buf())
    };

    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&config_path)
        .unwrap();

    let config_path = config_path.canonicalize().unwrap();

    let validator_config_path = config_path.join("0").join("node.yaml");
    let aptos_root_key_path = config_path.join("mint.key");

    let config = if try_load {
        NodeConfig::load(&validator_config_path).expect("Unable to load config:")
    } else {
        // Build a single validator network
        let mut maybe_config = PathBuf::from(&config_path);
        maybe_config.push("validator_node_template.yaml");
        let mut template = NodeConfig::load_config(maybe_config)
            .unwrap_or_else(|_| NodeConfig::default_for_validator());

        // enable REST and JSON-RPC API
        template.api.address = format!("0.0.0.0:{}", template.api.address.port())
            .parse()
            .unwrap();
        if lazy {
            template.consensus.mempool_poll_count = u64::MAX;
        }

        let builder = aptos_genesis_tool::validator_builder::ValidatorBuilder::new(
            &config_path,
            genesis_modules,
        )
        .template(template)
        .randomize_first_validator_ports(random_ports);

        let (root_keys, _genesis, genesis_waypoint, validators) = builder.build(rng).unwrap();

        let serialized_keys = bcs::to_bytes(&root_keys.root_key).unwrap();
        let mut key_file = std::fs::File::create(&aptos_root_key_path).unwrap();
        key_file.write_all(&serialized_keys).unwrap();

        // Build a waypoint file so that clients / docker can grab it easily
        let waypoint_file_path = config_path.join("waypoint.txt");
        std::io::Write::write_all(
            &mut std::fs::File::create(&waypoint_file_path).unwrap(),
            genesis_waypoint.to_string().as_bytes(),
        )
        .unwrap();

        validators[0].config.clone()
    };

    // Prepare log file since we cannot automatically route logs to stderr
    let log_file = config_path.join("validator.log");

    println!("Completed generating configuration:");
    println!("\tLog file: {:?}", log_file);
    println!("\tConfig path: {:?}", config_path);
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

    start(&config, Some(log_file))
}

// Fetch chain ID from on-chain resource
fn fetch_chain_id(db: &DbReaderWriter) -> ChainId {
    let db_state_view = db
        .reader
        .latest_state_checkpoint_view()
        .expect("[aptos-node] failed to create db state view");
    db_state_view
        .as_account_with_state_view(&aptos_root_address())
        .get_chain_id_resource()
        .expect("[aptos-node] failed to get chain ID resource")
        .expect("[aptos-node] missing chain ID resource")
        .chain_id()
}

fn setup_debug_interface(config: &NodeConfig, logger: Option<Arc<Logger>>) -> NodeDebugService {
    let addr = format!(
        "{}:{}",
        config.debug_interface.address, config.debug_interface.admission_control_node_debug_port,
    )
    .to_socket_addrs()
    .unwrap()
    .next()
    .unwrap();

    NodeDebugService::new(addr, logger, config)
}

fn create_state_sync_runtimes<M: MempoolNotificationSender + 'static>(
    node_config: &NodeConfig,
    storage_service_server_network_handles: Vec<StorageServiceNetworkEvents>,
    storage_service_client_network_handles: HashMap<
        NetworkId,
        storage_service_client::StorageServiceNetworkSender,
    >,
    state_sync_network_handles: Vec<(NetworkId, StateSyncSender, StateSyncEvents)>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
    mempool_notifier: M,
    consensus_listener: ConsensusNotificationListener,
    waypoint: Waypoint,
    event_subscription_service: EventSubscriptionService,
    db_rw: DbReaderWriter,
) -> StateSyncRuntimes {
    // Start the state sync storage service
    let storage_service_runtime = setup_state_sync_storage_service(
        node_config.state_sync.storage_service,
        storage_service_server_network_handles,
        &db_rw,
    );

    // Start the data client
    let (aptos_data_client, aptos_data_client_runtime) = setup_aptos_data_client(
        node_config.state_sync.storage_service,
        node_config.state_sync.aptos_data_client,
        node_config.base.clone(),
        storage_service_client_network_handles,
        peer_metadata_storage,
    );

    // Start the data streaming service
    let (streaming_service_client, streaming_service_runtime) = setup_data_streaming_service(
        node_config.state_sync.data_streaming_service,
        aptos_data_client.clone(),
    );

    // Create the chunk executor
    let chunk_executor = Arc::new(
        ChunkExecutor::<AptosVM>::new(db_rw.clone()).expect("Unable to create the chunk executor!"),
    );

    // Create the state sync multiplexer
    let state_sync_multiplexer = StateSyncMultiplexer::new(
        state_sync_network_handles,
        mempool_notifier,
        consensus_listener,
        db_rw,
        chunk_executor,
        node_config,
        waypoint,
        event_subscription_service,
        aptos_data_client,
        streaming_service_client,
    );

    // Create and return the new state sync handle
    StateSyncRuntimes::new(
        aptos_data_client_runtime,
        state_sync_multiplexer,
        storage_service_runtime,
        streaming_service_runtime,
    )
}

fn setup_data_streaming_service(
    config: DataStreamingServiceConfig,
    aptos_data_client: AptosNetDataClient,
) -> (StreamingServiceClient, Runtime) {
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
        .expect("Failed to create data streaming service!");
    streaming_service_runtime.spawn(data_streaming_service.start_service());

    (streaming_service_client, streaming_service_runtime)
}

fn setup_aptos_data_client(
    storage_service_config: StorageServiceConfig,
    aptos_data_client_config: AptosDataClientConfig,
    base_config: BaseConfig,
    network_handles: HashMap<NetworkId, storage_service_client::StorageServiceNetworkSender>,
    peer_metadata_storage: Arc<PeerMetadataStorage>,
) -> (AptosNetDataClient, Runtime) {
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
        .expect("Failed to create aptos data client!");

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

    (aptos_data_client, aptos_data_client_runtime)
}

fn setup_state_sync_storage_service(
    config: StorageServiceConfig,
    network_handles: Vec<StorageServiceNetworkEvents>,
    db_rw: &DbReaderWriter,
) -> Runtime {
    // Create a new state sync storage service runtime
    let storage_service_runtime = Builder::new_multi_thread()
        .thread_name("storage-service-server")
        .enable_all()
        .build()
        .expect("Failed to start the AptosNet storage-service runtime.");

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

    storage_service_runtime
}

// TODO(joshlind): clean me up and make everything configurable!
async fn periodic_telemetry_dump(node_config: NodeConfig, db: DbReaderWriter) {
    // Grab the peer id and chain id
    let peer_id = match node_config.peer_id() {
        Some(p) => p.to_string(),
        None => String::new(),
    };
    let chain_id = fetch_chain_id(&db).id().to_string(); // Get the chain_id as u8 for schema consistency

    // Send build information once, only on startup.
    send_build_information(peer_id.clone()).await;

    // Send system information every 5 minutes
    let mut system_information_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(NODE_SYS_INFO_FREQ_SECS),
    ))
    .fuse();

    // Send node metrics every 30 seconds
    let mut node_metrics_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(NODE_PUSH_METRICS_FREQ_SECS),
    ))
    .fuse();

    info!("periodic_telemetry_dump task started");
    loop {
        futures::select! {
            _ = system_information_interval.select_next_some() => {
                send_system_information(peer_id.clone()).await;
            }
            _ = node_metrics_interval.select_next_some() => {
                send_node_metrics(peer_id.clone(), chain_id.clone(), db.clone()).await;
            }
        }
    }
}

async fn send_build_information(peer_id: String) {
    tokio::spawn(async move {
        // Collect the build information
        let build_information = system_information::get_build_information();
        let build_information = convert_btree_to_hashmap(build_information);

        // Send the build information
        send_env_data(
            APTOS_NODE_BUILD_INFORMATION.to_string(),
            peer_id.to_string(),
            build_information,
        )
        .await;
    });
}

async fn send_system_information(peer_id: String) {
    tokio::spawn(async move {
        // Collect the system information
        let system_information = system_information::get_system_information();
        let system_information = convert_btree_to_hashmap(system_information);

        // Send the system information
        send_env_data(
            APTOS_NODE_SYSTEM_INFORMATION.to_string(),
            peer_id.to_string(),
            system_information,
        )
        .await;
    });
}

async fn send_node_metrics(peer_id: String, chain_id: String, db: DbReaderWriter) {
    tokio::spawn(async move {
        // Fetch the synced version
        let synced_version = (&*db.reader).fetch_synced_version().unwrap_or(0);

        // Send the node metrics
        let mut node_metrics: HashMap<String, String> = HashMap::new();
        node_metrics.insert(
            SYNCED_VERSION_METRIC.to_string(),
            synced_version.to_string(),
        );
        node_metrics.insert(CHAIN_ID_METRIC.to_string(), chain_id);
        node_metrics.insert(PEER_ID_METRIC.to_string(), peer_id.clone());
        send_env_data(APTOS_NODE_PUSH_METRICS.to_string(), peer_id, node_metrics).await;
    });
}

// TODO(joshlind): avoid the need to convert!
fn convert_btree_to_hashmap(btree: BTreeMap<String, String>) -> HashMap<String, String> {
    let mut hashmap = HashMap::new();
    for (key, value) in btree {
        hashmap.insert(key, value);
    }
    hashmap
}

async fn periodic_state_dump(node_config: NodeConfig, db: DbReaderWriter) {
    let args: Vec<String> = ::std::env::args().collect();

    // Once an hour
    let mut config_interval = IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(60 * 60),
    ))
    .fuse();
    // Once a minute
    let mut version_interval =
        IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(60))).fuse();

    info!("periodic_state_dump task started");

    loop {
        futures::select! {
            _ = config_interval.select_next_some() => {
                info!(config = node_config, args = args, "config and command line arguments");
            }
            _ = version_interval.select_next_some() => {
                let chain_id = fetch_chain_id(&db);
                let ledger_info = if let Ok(ledger_info) = db.reader.get_latest_ledger_info() {
                    ledger_info
                } else {
                    warn!("unable to query latest ledger info");
                    continue;
                };

                let latest_ledger_verion = ledger_info.ledger_info().version();
                let root_hash = ledger_info.ledger_info().transaction_accumulator_hash();

                info!(
                    chain_id = chain_id,
                    latest_ledger_verion = latest_ledger_verion,
                    root_hash = root_hash,
                    "latest ledger version and its corresponding root hash"
                );
            }
        }
    }
}

pub fn setup_environment(node_config: &NodeConfig, logger: Option<Arc<Logger>>) -> AptosHandle {
    let debug_if = setup_debug_interface(node_config, logger);

    let metrics_port = node_config.debug_interface.metrics_server_port;
    let metric_host = node_config.debug_interface.address.clone();
    thread::spawn(move || metric_server::start_server(metric_host, metrics_port));

    let mut instant = Instant::now();
    let (aptos_db, db_rw) = DbReaderWriter::wrap(
        AptosDB::open(
            &node_config.storage.dir(),
            false, /* readonly */
            node_config.storage.storage_pruner_config,
            node_config.storage.rocksdb_config,
        )
        .expect("DB should open."),
    );
    let _simple_storage_service = start_storage_service_with_db(node_config, Arc::clone(&aptos_db));
    let backup_service = start_backup_service(
        node_config.storage.backup_service_address,
        Arc::clone(&aptos_db),
    );

    let genesis_waypoint = node_config.base.waypoint.genesis_waypoint();
    // if there's genesis txn and waypoint, commit it if the result matches.
    if let Some(genesis) = get_genesis_txn(node_config) {
        maybe_bootstrap::<AptosVM>(&db_rw, genesis, genesis_waypoint)
            .expect("Db-bootstrapper should not fail.");
    } else {
        info!("Genesis txn not provided, it's fine if you don't expect to apply it otherwise please double check config");
    }
    AptosVM::set_concurrency_level_once(node_config.execution.concurrency_level as usize);

    debug!(
        "Storage service started in {} ms",
        instant.elapsed().as_millis()
    );

    let chain_id = fetch_chain_id(&db_rw);
    let mut network_runtimes = vec![];
    let mut state_sync_network_handles = vec![];
    let mut mempool_network_handles = vec![];
    let mut consensus_network_handles = None;
    let mut storage_service_server_network_handles = vec![];
    let mut storage_service_client_network_handles = HashMap::new();

    // Create an event subscription service so that components can be notified of events and reconfigs
    let mut event_subscription_service = EventSubscriptionService::new(
        ON_CHAIN_CONFIG_REGISTRY,
        Arc::new(RwLock::new(db_rw.clone())),
    );
    let mempool_reconfig_subscription = event_subscription_service
        .subscribe_to_reconfigurations()
        .unwrap();

    // Create a consensus subscription for reconfiguration events (if this node is a validator).
    let consensus_reconfig_subscription = if node_config.base.role.is_validator() {
        Some(
            event_subscription_service
                .subscribe_to_reconfigurations()
                .unwrap(),
        )
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
        let runtime = Builder::new_multi_thread()
            .thread_name(format!("network-{}", network_config.network_id))
            .enable_all()
            .build()
            .expect("Failed to start runtime. Won't be able to start networking.");

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

        // Create the endpoints to connect the Network to State Sync.
        let (state_sync_sender, state_sync_events) =
            network_builder.add_p2p_service(&state_sync_v1_network_config());
        state_sync_network_handles.push((network_id, state_sync_sender, state_sync_events));

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
            node_config.state_sync.client_commit_timeout_ms,
        );

    // Create the state sync runtimes
    let state_sync_runtimes = create_state_sync_runtimes(
        node_config,
        storage_service_server_network_handles,
        storage_service_client_network_handles,
        state_sync_network_handles,
        peer_metadata_storage.clone(),
        mempool_notifier,
        consensus_listener,
        genesis_waypoint,
        event_subscription_service,
        db_rw.clone(),
    );

    let (mp_client_sender, mp_client_events) = channel(AC_SMP_CHANNEL_BUFFER_SIZE);

    let api_runtime = bootstrap_api(node_config, chain_id, aptos_db, mp_client_sender).unwrap();

    let mut consensus_runtime = None;
    let (consensus_to_mempool_sender, consensus_requests) = channel(INTRA_NODE_CHANNEL_BUFFER_SIZE);

    instant = Instant::now();
    let mempool = aptos_mempool::bootstrap(
        node_config,
        Arc::clone(&db_rw.reader),
        mempool_network_handles,
        mp_client_events,
        consensus_requests,
        mempool_listener,
        mempool_reconfig_subscription,
        peer_metadata_storage.clone(),
    );
    debug!("Mempool started in {} ms", instant.elapsed().as_millis());

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
            node_config,
            consensus_network_sender,
            consensus_network_events,
            Arc::new(consensus_notifier),
            consensus_to_mempool_sender,
            db_rw.clone(),
            consensus_reconfig_subscription
                .expect("Consensus requires a reconfiguration subscription!"),
            peer_metadata_storage,
        ));
        debug!("Consensus started in {} ms", instant.elapsed().as_millis());
    }

    // Spawn a task which will periodically dump some interesting state
    debug_if
        .runtime()
        .handle()
        .spawn(periodic_state_dump(node_config.to_owned(), db_rw.clone()));

    let telemery_runtime = Builder::new_multi_thread()
        .thread_name("aptos-telemetry")
        .enable_all()
        .build()
        .expect("Failed to create aptos telemetry runtime!");

    // TODO(joshlind): clean this up!
    if !aptos_telemetry::is_disabled() {
        telemery_runtime
            .handle()
            .spawn(periodic_telemetry_dump(node_config.to_owned(), db_rw));
    }

    AptosHandle {
        _api: api_runtime,
        _backup: backup_service,
        _consensus_runtime: consensus_runtime,
        _debug: debug_if,
        _mempool: mempool,
        _network_runtimes: network_runtimes,
        _state_sync_runtimes: state_sync_runtimes,
        _telemetry_runtime: telemery_runtime,
    }
}
