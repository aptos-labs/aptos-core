use anyhow::Result;
use aptos_logger::info;
use aptos_types::waypoint::Waypoint;
use aptos_config::config::NodeConfig;
use aptos_storage_interface::DbReaderWriter;
use aptos_state_sync_driver::driver_factory::StateSyncRuntimes;
use aptos_time_service::TimeService;
use tokio::runtime::Runtime;

pub fn sync_fullnode() -> Result<()> {
    // Initialize the logger
    aptos_logger::Logger::new().init();

    // Load the node configuration
    let config_path = std::env::var("NODE_CONFIG_PATH")
        .expect("NODE_CONFIG_PATH environment variable must be set");
    let node_config = NodeConfig::load(&config_path)
        .expect("Failed to load node configuration");

    // Initialize the database
    let (db_rw, _, waypoint, _, _) = aptos_node::storage::initialize_database_and_checkpoints(&mut node_config.clone())
        .expect("Failed to initialize database");

    // Create the state sync runtimes
    let state_sync_runtimes = StateSyncRuntimes::new(
        Runtime::new().expect("Failed to create Tokio runtime"),
        db_rw.clone(),
        waypoint,
        node_config.state_sync.clone(),
        TimeService::real(),
    );

    // Start the state sync process
    state_sync_runtimes.block_until_initialized();
    info!("State sync initialization complete.");

    Ok(())
}
