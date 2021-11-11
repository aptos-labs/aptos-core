// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use consensus_notifications::ConsensusNotificationListener;
use diem_config::{config::NodeConfig, network_id::NetworkId};
use diem_types::{move_resource::MoveStorage, protocol_spec::DpnProto, waypoint::Waypoint};
use event_notifications::{EventNotificationSender, EventSubscriptionService};
use executor_types::ChunkExecutor;
use futures::executor::block_on;
use mempool_notifications::MempoolNotificationSender;
use network::protocols::network::AppConfig;
use state_sync_v1::{
    bootstrapper::StateSyncBootstrapper,
    network::{StateSyncEvents, StateSyncSender},
};
use std::sync::Arc;
use storage_interface::DbReader;

/// A multiplexer allowing multiple versions of state sync to operate
/// concurrently (i.e., state sync v1 and state sync v2).
pub struct StateSyncMultiplexer {
    state_sync_v1: StateSyncBootstrapper,
}

impl StateSyncMultiplexer {
    pub fn new<M: MempoolNotificationSender + 'static>(
        network: Vec<(NetworkId, StateSyncSender, StateSyncEvents)>,
        mempool_notifier: M,
        consensus_listener: ConsensusNotificationListener,
        storage: Arc<dyn DbReader<DpnProto>>,
        executor: Box<dyn ChunkExecutor>,
        node_config: &NodeConfig,
        waypoint: Waypoint,
        mut event_subscription_service: EventSubscriptionService,
    ) -> Self {
        // Notify subscribers of the initial on-chain config values
        match (&*storage).fetch_synced_version() {
            Ok(synced_version) => {
                if let Err(error) =
                    event_subscription_service.notify_initial_configs(synced_version)
                {
                    panic!(
                        "Failed to notify subscribers of initial on-chain configs: {:?}",
                        error
                    )
                }
            }
            Err(error) => panic!("Failed to fetch the initial synced version: {:?}", error),
        }

        // TODO(joshlind): use configs to determine the state sync v1 mode
        let state_sync_v1_read_only = false;

        // Create a state sync v1 bootstrapper
        let state_sync_bootstrapper = StateSyncBootstrapper::bootstrap(
            network,
            mempool_notifier,
            consensus_listener,
            storage,
            executor,
            node_config,
            waypoint,
            event_subscription_service,
            state_sync_v1_read_only,
        );

        Self {
            state_sync_v1: state_sync_bootstrapper,
        }
    }

    pub fn block_until_initialized(&self) {
        let state_sync_v1_client = self.state_sync_v1.create_client();
        block_on(state_sync_v1_client.wait_until_initialized())
            .expect("State sync v1 initialization failure");
    }
}

/// Configuration for the network endpoints to support state sync.
pub fn state_sync_v1_network_config() -> AppConfig {
    state_sync_v1::network::network_endpoint_config()
}

#[cfg(any(test, feature = "fuzzing"))]
mod tests {
    use crate::StateSyncMultiplexer;
    use consensus_notifications::new_consensus_notifier_listener_pair;
    use diem_config::{config::RocksdbConfig, utils::get_genesis_txn};
    use diem_crypto::HashValue;
    use diem_genesis_tool::test_config;
    use diem_infallible::RwLock;
    use diem_temppath::TempPath;
    use diem_types::{
        block_info::BlockInfo, ledger_info::LedgerInfo, on_chain_config::ON_CHAIN_CONFIG_REGISTRY,
        protocol_spec::DpnProto, waypoint::Waypoint,
    };
    use diem_vm::DiemVM;
    use diemdb::DiemDB;
    use event_notifications::EventSubscriptionService;
    use executor::Executor;
    use executor_test_helpers::bootstrap_genesis;
    use futures::{FutureExt, StreamExt};
    use mempool_notifications::new_mempool_notifier_listener_pair;
    use std::sync::Arc;
    use storage_interface::default_protocol::DbReaderWriter;

    #[test]
    fn test_new_initialized_configs() {
        // Create a test database
        let tmp_dir = TempPath::new();
        let db = DiemDB::open(&tmp_dir, false, None, RocksdbConfig::default(), true).unwrap();
        let (_, db_rw) = DbReaderWriter::wrap(db);

        // Bootstrap the database
        let (node_config, _) = test_config();
        bootstrap_genesis::<DiemVM>(&db_rw, get_genesis_txn(&node_config).unwrap()).unwrap();

        // Create mempool and consensus notifiers
        let (mempool_notifier, _) = new_mempool_notifier_listener_pair();
        let (_, consensus_listener) = new_consensus_notifier_listener_pair(0);

        // Create the event subscription service and a reconfig subscriber
        let mut event_subscription_service = EventSubscriptionService::new(
            ON_CHAIN_CONFIG_REGISTRY,
            Arc::new(RwLock::new(db_rw.clone())),
        );
        let mut reconfiguration_subscriber = event_subscription_service
            .subscribe_to_reconfigurations()
            .unwrap();

        // Create the multiplexer
        let _ = StateSyncMultiplexer::new(
            vec![],
            mempool_notifier,
            consensus_listener,
            Arc::clone(&db_rw.reader),
            Box::new(Executor::<DpnProto, DiemVM>::new(db_rw.clone())),
            &node_config,
            Waypoint::new_any(&LedgerInfo::new(BlockInfo::empty(), HashValue::random())),
            event_subscription_service,
        );

        // Verify the initial configs were notified
        assert!(reconfiguration_subscriber
            .select_next_some()
            .now_or_never()
            .is_some());
    }
}
