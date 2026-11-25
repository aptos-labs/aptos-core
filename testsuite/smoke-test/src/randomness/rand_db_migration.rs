// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Manual migration validation; remove after the follow-up release's rollout.
//!
//! ```sh
//! RANDDB_UPGRADE_OLD_BIN=/path/to/legacy/aptos-node \
//! RANDDB_UPGRADE_NEW_BIN=/path/to/new/aptos-node \
//! RUST_MIN_STACK=67108864 \
//! cargo test -p smoke-test randomness::rand_db_migration --locked -- \
//!     --ignored --test-threads=1 --nocapture
//! ```
//!
//! Without both binary overrides, Forge builds the pinned revision and workspace from a full checkout.

use crate::randomness::e2e_basic_consumption::publish_on_chain_dice_module;
use aptos::{
    common::types::{GasOptions, TransactionSummary},
    test::CliTestFramework,
};
use aptos_forge::{
    ActiveNodesGuard, Factory, LocalFactory, LocalSwarm, LocalVersion, Node, NodeExt, Swarm,
    SwarmExt, Version,
};
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use once_cell::sync::Lazy;
use rand::rngs::OsRng;
use std::{env, fs, num::NonZeroUsize, path::PathBuf, str::FromStr, sync::Arc, time::Duration};

const NUM_VALIDATORS: usize = 5;
const TIMEOUT: Duration = Duration::from_secs(120);
// Leave time for five sequential restarts without crossing the epoch boundary.
const EPOCH_DURATION_SECS: u64 = 180;
const EPOCH_TIMEOUT: Duration = Duration::from_secs(EPOCH_DURATION_SECS + TIMEOUT.as_secs());
const LEGACY_REVISION: &str = "7b23ba0a947166c6c2fccc32608a345942f209a3";

static FACTORY: Lazy<LocalFactory> = Lazy::new(|| {
    match (
        env::var_os("RANDDB_UPGRADE_OLD_BIN"),
        env::var_os("RANDDB_UPGRADE_NEW_BIN"),
    ) {
        (Some(old), Some(new)) => {
            let versions = [
                (0, LEGACY_REVISION, PathBuf::from(old)),
                (1, "phase1", PathBuf::from(new)),
            ]
            .into_iter()
            .map(|(order, name, path)| {
                assert!(
                    path.is_file(),
                    "missing validator binary: {}",
                    path.display()
                );
                let version = Version::new(order, name.to_owned());
                (version.clone(), LocalVersion::new(path, version))
            })
            .collect();
            LocalFactory::new(versions, None)
        },
        (None, None) => LocalFactory::with_revision_and_workspace(LEGACY_REVISION).unwrap(),
        _ => panic!("set both RANDDB_UPGRADE_OLD_BIN and RANDDB_UPGRADE_NEW_BIN"),
    }
});

struct MigrationSwarm {
    swarm: LocalSwarm,
    cli: CliTestFramework,
    peers: Vec<AccountAddress>,
    upgrade_version: Version,
    epoch: u64,
    dice_roll_function: MemberId,
}

impl MigrationSwarm {
    async fn new() -> Self {
        aptos_logger::Logger::new().init();
        let old_version = FACTORY.versions().min().unwrap();
        let upgrade_version = FACTORY.versions().max().unwrap();
        assert_ne!(old_version, upgrade_version);
        static ACTIVE_NODES: Lazy<Arc<Mutex<usize>>> = Lazy::new(|| Arc::new(Mutex::new(0)));
        let guard = ActiveNodesGuard::grab(NUM_VALIDATORS * 2, ACTIVE_NODES.clone()).await;
        let swarm = FACTORY
            .new_swarm_with_version(
                OsRng,
                NonZeroUsize::new(NUM_VALIDATORS).unwrap(),
                0,
                &old_version,
                Some(aptos_cached_packages::head_release_bundle().clone()),
                None,
                None,
                Some(Arc::new(|_, stake| {
                    *stake = 100_000_000_000_000;
                })),
                Some(Arc::new(|conf| {
                    conf.epoch_duration_secs = EPOCH_DURATION_SECS;
                    conf.consensus_config.enable_validator_txns();
                    conf.consensus_config.disable_rand_check();
                    conf.randomness_config_override =
                        Some(OnChainRandomnessConfig::default_enabled());
                    conf.chunky_dkg_config_override =
                        Some(OnChainChunkyDKGConfig::default_disabled());
                })),
                guard,
            )
            .await
            .unwrap();
        swarm
            .wait_for_all_nodes_to_catchup_to_epoch(2, EPOCH_TIMEOUT)
            .await
            .unwrap();
        let peers: Vec<_> = swarm.validators().map(|node| node.peer_id()).collect();
        assert!(swarm.validators().all(|node| node.version() == old_version));
        let node = swarm.validator(peers[0]).unwrap();
        let endpoint = node.rest_api_endpoint();
        let mut cli = CliTestFramework::new(endpoint.clone(), endpoint, 0).await;
        let root_address = swarm.chain_info().root_account().address();
        assert_eq!(
            cli.add_account_with_address_to_cli(swarm.root_key(), root_address),
            0
        );
        publish_on_chain_dice_module(&mut cli, 0).await;
        let epoch = node
            .rest_client()
            .get_ledger_information()
            .await
            .unwrap()
            .inner()
            .epoch;
        let dice_roll_function =
            MemberId::from_str(&format!("{}::dice::roll", root_address.to_hex_literal())).unwrap();
        let mut test = Self {
            swarm,
            cli,
            peers,
            upgrade_version,
            epoch,
            dice_roll_function,
        };
        // Start the rollout at a fresh epoch so the same-epoch batch has its full time budget.
        test.wait_for_next_epoch().await;
        test
    }

    async fn current_epoch(&self) -> u64 {
        self.swarm
            .validator(self.peers[0])
            .unwrap()
            .rest_client()
            .get_ledger_information()
            .await
            .unwrap()
            .inner()
            .epoch
    }

    async fn upgrade(&mut self, index: usize) {
        assert_eq!(self.current_epoch().await, self.epoch);
        self.swarm
            .upgrade_validator(self.peers[index], &self.upgrade_version)
            .await
            .unwrap();
        self.swarm.wait_all_alive(TIMEOUT).await.unwrap();
        self.swarm
            .wait_for_all_nodes_to_catchup(TIMEOUT)
            .await
            .unwrap();
        assert_eq!(
            self.swarm.validator(self.peers[index]).unwrap().version(),
            self.upgrade_version
        );
        assert_eq!(
            self.current_epoch().await,
            self.epoch,
            "upgrade crossed the epoch boundary"
        );
        info!(
            "Phase1 rollout: validator {} upgraded in epoch {}; original data directory retained",
            index, self.epoch
        );
    }

    async fn wait_for_next_epoch(&mut self) {
        self.epoch += 1;
        self.swarm
            .wait_for_all_nodes_to_catchup_to_epoch(self.epoch, EPOCH_TIMEOUT)
            .await
            .unwrap();
        assert_eq!(self.current_epoch().await, self.epoch);
        info!("Phase1 rollout: reached natural epoch {}", self.epoch);
    }

    async fn restart_validator_without_rand_db(&mut self, index: usize) {
        assert_eq!(self.current_epoch().await, self.epoch);
        let node = self.swarm.validator(self.peers[index]).unwrap();
        assert_eq!(node.version(), self.upgrade_version);
        node.stop();
        fs::remove_dir_all(node.config().storage.dir().join("rand_db")).unwrap();
        node.start().unwrap();
        self.swarm.wait_all_alive(TIMEOUT).await.unwrap();
        info!(
            "RandDB retirement: validator {} restarted without RandDB in epoch {}",
            index, self.epoch
        );
    }

    async fn submit_dice_roll(&self) -> TransactionSummary {
        self.cli
            .run_function(
                0,
                Some(GasOptions {
                    gas_unit_price: Some(100),
                    max_gas: Some(10_000),
                    expiration_secs: 120,
                }),
                self.dice_roll_function.clone(),
                vec![],
                vec![],
            )
            .await
            .unwrap()
    }
}

#[tokio::test]
#[ignore = "manual cross-version RandDB migration validation"]
async fn one_upgrade_per_epoch_then_retire_rand_db() {
    let mut test = MigrationSwarm::new().await;
    let transaction = test.submit_dice_roll().await;
    assert_eq!(transaction.success, Some(true));
    test.swarm
        .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
        .await
        .unwrap();

    for index in 0..4 {
        test.upgrade(index).await;
        let transaction = test.submit_dice_roll().await;
        assert_eq!(transaction.success, Some(true));
        test.swarm
            .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
            .await
            .unwrap();

        test.wait_for_next_epoch().await;
        let transaction = test.submit_dice_roll().await;
        assert_eq!(transaction.success, Some(true));
        test.swarm
            .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
            .await
            .unwrap();
    }

    // The last validator retains RandDB during its binary upgrade too.
    test.upgrade(4).await;
    test.wait_for_next_epoch().await;
    assert!(test
        .swarm
        .validators()
        .all(|node| node.version() == test.upgrade_version));

    // Other validators retain their certificates while each old RandDB is discarded.
    for index in 0..NUM_VALIDATORS {
        test.restart_validator_without_rand_db(index).await;
        let transaction = test.submit_dice_roll().await;
        assert_eq!(transaction.success, Some(true));
        test.swarm
            .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
            .await
            .unwrap();
        assert_eq!(
            test.current_epoch().await,
            test.epoch,
            "RandDB recovery waited for another epoch"
        );
        info!(
            "RandDB retirement: validator {} recovered; all nodes committed version {} in epoch {}",
            index,
            transaction.version.unwrap(),
            test.epoch
        );
    }
}

#[tokio::test]
#[ignore = "manual cross-version RandDB migration validation"]
async fn majority_upgrade_in_one_epoch_then_retire_rand_db() {
    let mut test = MigrationSwarm::new().await;
    let transaction = test.submit_dice_roll().await;
    assert_eq!(transaction.success, Some(true));
    test.swarm
        .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
        .await
        .unwrap();

    for index in 0..4 {
        test.upgrade(index).await;
    }
    let transaction = test.submit_dice_roll().await;
    assert_eq!(transaction.success, Some(true));
    test.swarm
        .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
        .await
        .unwrap();

    test.wait_for_next_epoch().await;
    let transaction = test.submit_dice_roll().await;
    assert_eq!(transaction.success, Some(true));
    test.swarm
        .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
        .await
        .unwrap();

    // The last validator retains RandDB during its binary upgrade too.
    test.upgrade(4).await;
    test.wait_for_next_epoch().await;
    assert!(test
        .swarm
        .validators()
        .all(|node| node.version() == test.upgrade_version));

    // Other validators retain their certificates while each old RandDB is discarded.
    for index in 0..NUM_VALIDATORS {
        test.restart_validator_without_rand_db(index).await;
        let transaction = test.submit_dice_roll().await;
        assert_eq!(transaction.success, Some(true));
        test.swarm
            .wait_for_all_nodes_to_catchup_to_version(transaction.version.unwrap(), TIMEOUT)
            .await
            .unwrap();
        assert_eq!(
            test.current_epoch().await,
            test.epoch,
            "RandDB recovery waited for another epoch"
        );
        info!(
            "RandDB retirement: validator {} recovered; all nodes committed version {} in epoch {}",
            index,
            transaction.version.unwrap(),
            test.epoch
        );
    }
}
