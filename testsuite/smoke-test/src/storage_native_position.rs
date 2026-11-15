// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native positions across backup/restore and across enabling them on a running
//! chain. Position state roots are committed in `TransactionInfo`, so a node
//! that catches up has recomputed the same roots; the tests also compare its
//! positions with a node that never stopped and with what they wrote.

use crate::{
    smoke_test_environment::SwarmBuilder,
    storage::{db_backup_with_args, db_restore_with_args},
    utils::{
        create_test_accounts, get_on_chain_resource, swarm_utils::insert_waypoint, transfer_coins,
        MAX_CATCH_UP_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
    workspace_builder,
};
use aptos_config::config::{
    HotStateConfig, NodeConfig, RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::{native_state_reader::NativeStateReader, AptosDB};
use aptos_forge::{AptosPublicInfo, LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_logger::info;
use aptos_sdk::{
    move_types::{identifier::Identifier, language_storage::ModuleId},
    types::LocalAccount,
};
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{FeatureFlag, Features},
    state_store::native_position::NativePosition,
    transaction::{EntryFunction, Script},
    PeerId,
};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

const NUM_ACCOUNTS: u8 = 8;
const NUM_MARKETS: u8 = 4;

fn enable_trading_native(config: &mut NodeConfig) {
    config.storage.rocksdb_configs.enable_trading_native = true;
}

fn address(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[0] = 0xEE;
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

async fn call(
    info: &mut AptosPublicInfo,
    exchange: AccountAddress,
    signer: &LocalAccount,
    function: &str,
    args: Vec<Vec<u8>>,
) {
    let module = ModuleId::new(exchange, Identifier::new("positions").unwrap());
    let txn = signer.sign_with_transaction_builder(info.transaction_factory().entry_function(
        EntryFunction::new(module, Identifier::new(function).unwrap(), vec![], args),
    ));
    info.client().submit_and_wait(&txn).await.unwrap();
}

/// Position sizes written so far, keyed by `(account, market)`.
type Expected = BTreeMap<(AccountAddress, AccountAddress), u64>;

async fn set(
    info: &mut AptosPublicInfo,
    exchange: &LocalAccount,
    expected: &mut Expected,
    account: u8,
    markets: &[u8],
    size: u64,
) {
    let markets: Vec<_> = markets.iter().copied().map(address).collect();
    call(info, exchange.address(), exchange, "set_positions", vec![
        bcs::to_bytes(&address(100 + account)).unwrap(),
        bcs::to_bytes(&markets).unwrap(),
        bcs::to_bytes(&size).unwrap(),
    ])
    .await;
    for market in markets {
        expected.insert((address(100 + account), market), size);
    }
}

/// Sets every account's position on every market to `size`.
async fn set_all(
    info: &mut AptosPublicInfo,
    exchange: &LocalAccount,
    expected: &mut Expected,
    size: u64,
) {
    let markets: Vec<_> = (0..NUM_MARKETS).collect();
    for account in 0..NUM_ACCOUNTS {
        set(info, exchange, expected, account, &markets, size).await;
    }
}

async fn delete(
    info: &mut AptosPublicInfo,
    exchange: &LocalAccount,
    expected: &mut Expected,
    account: u8,
    market: u8,
) {
    call(info, exchange.address(), exchange, "delete_position", vec![
        bcs::to_bytes(&address(100 + account)).unwrap(),
        bcs::to_bytes(&address(market)).unwrap(),
    ])
    .await;
    expected.remove(&(address(100 + account), address(market)));
}

/// Every position of `exchange` in the DB at `db_dir`, keyed by `(account, market)`.
fn read_positions(
    db_dir: &Path,
    exchange: AccountAddress,
) -> BTreeMap<(AccountAddress, AccountAddress), NativePosition> {
    let db = AptosDB::open(
        StorageDirPaths::from_path(db_dir),
        false, /* readonly */
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfigs {
            enable_trading_native: true,
            enable_storage_sharding: true,
            ..Default::default()
        },
        BUFFERED_STATE_TARGET_ITEMS,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
        None,
        HotStateConfig::default(),
    )
    .unwrap();
    let reader = db.native_state_reader().unwrap();
    reader
        .iter_position_accounts_for_exchange(exchange)
        .into_iter()
        .flat_map(|account| {
            reader
                .get_account_positions(exchange, account)
                .into_iter()
                .map(move |(market, position)| ((account, market), position))
        })
        .collect()
}

const POSITION_FEATURES: [FeatureFlag; 3] = [
    FeatureFlag::TRADING_NATIVE,
    FeatureFlag::NATIVE_POSITION,
    FeatureFlag::COMPUTE_TRADING_NATIVE_STATE_ROOTS,
];

/// `TRANSACTION_INFO_V1` and `HOTNESS_IN_EPILOGUE` on, plus the position
/// features when `with_positions`.
fn genesis_features(with_positions: bool) -> Features {
    let mut features = Features::default();
    features.enable(FeatureFlag::TRANSACTION_INFO_V1);
    features.enable(FeatureFlag::HOTNESS_IN_EPILOGUE);
    for flag in POSITION_FEATURES {
        if with_positions {
            features.enable(flag);
        } else {
            features.disable(flag);
        }
    }
    features
}

/// Builds the position package for `exchange`.
fn build_package(exchange: AccountAddress) -> BuiltPackage {
    let mut build_options = BuildOptions::default();
    build_options
        .named_addresses
        .insert("exchange".to_string(), exchange);
    // Compiling AptosExperimental overflows a test thread's default stack.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || {
            BuiltPackage::build(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("src/storage_native_position"),
                build_options,
            )
        })
        .unwrap()
        .join()
        .unwrap()
        .unwrap()
}

/// Publishes `package` from `exchange` and registers it as an exchange.
async fn publish_and_register(
    info: &mut AptosPublicInfo,
    exchange: &LocalAccount,
    package: &BuiltPackage,
) {
    let publish = exchange.sign_with_transaction_builder(info.transaction_factory().payload(
        aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&package.extract_metadata().unwrap()).unwrap(),
            package.extract_code(),
        ),
    ));
    info.client().submit_and_wait(&publish).await.unwrap();
    let root = info.root_account();
    call(info, exchange.address(), &root, "register_exchange", vec![
        bcs::to_bytes(&exchange.address()).unwrap(),
    ])
    .await;
    call(info, exchange.address(), exchange, "init_cap", vec![]).await;
}

async fn new_exchange(info: &mut AptosPublicInfo) -> LocalAccount {
    info.create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap()
}

/// Backs up from `source` (replay-verifying the backup) and returns the backup.
fn backup(swarm: &LocalSwarm, source: PeerId, target_version: u64) -> aptos_temppath::TempPath {
    let port = swarm
        .validator(source)
        .unwrap()
        .config()
        .storage
        .backup_service_address
        .port();
    let (backup_path, snapshot_version) =
        db_backup_with_args(port, 1, target_version, 200, 2, &[], &[
            "--enable-trading-native",
        ]);
    info!("backup has a state snapshot at version {snapshot_version}");
    backup_path
}

/// Wipes `node`, restores it from `backup_path` and restarts it.
async fn restore_node(swarm: &mut LocalSwarm, node: PeerId, backup_path: &Path) {
    swarm.validator_mut(node).unwrap().stop();
    let config_path = swarm.validator(node).unwrap().config_path();
    let mut config = swarm.validator(node).unwrap().config().clone();
    let genesis_waypoint = config.base.waypoint.genesis_waypoint();
    insert_waypoint(&mut config, genesis_waypoint);
    config.save_to_path(config_path).unwrap();
    let db_dir = config.storage.dir();
    fs::remove_dir_all(&db_dir).unwrap();
    db_restore_with_args(backup_path, &db_dir, &[], None, &[
        "--enable-trading-native",
    ]);
    start_and_wait(swarm, node).await;
}

/// Restarts `node`, after applying `update` to its config.
async fn restart_node(swarm: &mut LocalSwarm, node: PeerId, update: impl FnOnce(&mut NodeConfig)) {
    let validator = swarm.validator_mut(node).unwrap();
    validator.stop();
    update(validator.config_mut());
    let mut config = validator.config().clone();
    config.save_to_path(validator.config_path()).unwrap();
    start_and_wait(swarm, node).await;
}

async fn start_and_wait(swarm: &mut LocalSwarm, node: PeerId) {
    swarm.validator_mut(node).unwrap().start().unwrap();
    swarm
        .validator_mut(node)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();
}

/// Stops `nodes` and requires each to hold exactly `expected`.
fn assert_positions(
    swarm: &mut LocalSwarm,
    nodes: &[PeerId],
    exchange: AccountAddress,
    expected: &Expected,
) {
    for node in nodes {
        swarm.validator_mut(*node).unwrap().stop();
    }
    for node in nodes {
        let positions = read_positions(
            &swarm.validator(*node).unwrap().config().storage.dir(),
            exchange,
        );
        let sizes: Expected = positions
            .into_iter()
            .map(|(key, position)| match position {
                NativePosition::PerpV1 { size, .. } => (key, size),
            })
            .collect();
        assert_eq!(&sizes, expected, "positions on {node}");
    }
}

/// Turns the position features on through governance (the package's
/// `enable_positions` script) and waits for the next epoch to apply them.
async fn enable_positions_via_governance(info: &mut AptosPublicInfo, package: &BuiltPackage) {
    let [script] = package.extract_script_code().try_into().unwrap();
    let root = info.root_account();
    let txn = root.sign_with_transaction_builder(info.transaction_factory().script(Script::new(
        script,
        vec![],
        vec![],
    )));
    info.client().submit_and_wait(&txn).await.unwrap();

    let deadline = Instant::now() + Duration::from_secs(MAX_CATCH_UP_WAIT_SECS);
    loop {
        let features = get_on_chain_resource::<Features>(info.client()).await;
        if POSITION_FEATURES
            .iter()
            .all(|flag| features.is_enabled(*flag))
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "position features were not enabled in time"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn latest_version(info: &AptosPublicInfo) -> u64 {
    info.client()
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version
}

/// Positions written, overwritten and deleted before each epoch ending, so state
/// snapshots carry positions and replay past them applies more.
async fn write_across_epochs(
    info: &mut AptosPublicInfo,
    exchange: &LocalAccount,
    expected: &mut Expected,
    epochs: u64,
) {
    for epoch in 0..epochs {
        set_all(info, exchange, expected, 100 + epoch).await;
        delete(
            info,
            exchange,
            expected,
            epoch as u8 % NUM_ACCOUNTS,
            epoch as u8 % NUM_MARKETS,
        )
        .await;
        info.reconfig().await;
    }
}

#[tokio::test]
async fn test_db_restore_with_native_positions() {
    ::aptos_logger::Logger::new().init();
    workspace_builder::get_bin("aptos-debugger");

    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| enable_trading_native(config)))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.initial_features_override = Some(genesis_features(true));
        }))
        .build()
        .await;
    let nodes = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let mut info = swarm.aptos_public_info();

    info!("---------- 1. publishing the position package and registering the exchange.");
    let exchange = new_exchange(&mut info).await;
    publish_and_register(&mut info, &exchange, &build_package(exchange.address())).await;

    info!("---------- 2. writing positions across several epochs.");
    let mut expected = Expected::new();
    write_across_epochs(&mut info, &exchange, &mut expected, 6).await;
    delete(&mut info, &exchange, &mut expected, 1, 2).await;
    info.reconfig().await;

    info!("---------- 3. backing up from node 1, restoring node 0.");
    let backup_path = backup(&swarm, nodes[1], latest_version(&info).await);
    restore_node(&mut swarm, nodes[0], backup_path.path()).await;

    info!("---------- 4. writing past the backup; catching up proves node 0's roots match.");
    for account in (0..NUM_ACCOUNTS).filter(|account| *account != 1) {
        set(&mut info, &exchange, &mut expected, account, &[2], 1234).await;
    }
    delete(&mut info, &exchange, &mut expected, 3, 0).await;
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    info!("---------- 5. comparing positions on node 0 and node 1.");
    assert_positions(&mut swarm, &nodes[..2], exchange.address(), &expected);
}

/// A chain that ran without native positions enables them: a rolling restart
/// turns position storage on over existing DBs, governance then turns the
/// features on, and positions survive a restart and a backup/restore taken after
/// the switch.
#[tokio::test]
async fn test_enable_native_positions_on_running_chain() {
    ::aptos_logger::Logger::new().init();
    workspace_builder::get_bin("aptos-debugger");

    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.initial_features_override = Some(genesis_features(false));
        }))
        .build()
        .await;
    let nodes = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let mut info = swarm.aptos_public_info();
    let transaction_factory = swarm.chain_info().transaction_factory();
    let (mut sender, receiver) = create_test_accounts(&mut swarm).await;

    info!("---------- 1. history without positions.");
    for _ in 0..3 {
        for _ in 0..5 {
            transfer_coins(
                info.client(),
                &transaction_factory,
                &mut sender,
                &receiver,
                1,
            )
            .await;
        }
        info.reconfig().await;
    }

    info!("---------- 2. rolling restart with position storage on.");
    for node in &nodes {
        restart_node(&mut swarm, *node, enable_trading_native).await;
        transfer_coins(
            info.client(),
            &transaction_factory,
            &mut sender,
            &receiver,
            1,
        )
        .await;
        swarm
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
            .await
            .unwrap();
    }

    info!("---------- 3. enabling the position features through governance.");
    let exchange = new_exchange(&mut info).await;
    let package = build_package(exchange.address());
    enable_positions_via_governance(&mut info, &package).await;
    publish_and_register(&mut info, &exchange, &package).await;
    let mut expected = Expected::new();
    write_across_epochs(&mut info, &exchange, &mut expected, 3).await;

    info!("---------- 4. restarting node 2 over its position store.");
    restart_node(&mut swarm, nodes[2], |_| {}).await;
    set_all(&mut info, &exchange, &mut expected, 500).await;
    delete(&mut info, &exchange, &mut expected, 6, 1).await;
    info.reconfig().await;

    info!("---------- 5. backing up from node 1, restoring node 0.");
    let backup_path = backup(&swarm, nodes[1], latest_version(&info).await);
    restore_node(&mut swarm, nodes[0], backup_path.path()).await;
    set(&mut info, &exchange, &mut expected, 4, &[0, 3], 777).await;
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    info!("---------- 6. comparing positions on every node.");
    assert_positions(&mut swarm, &nodes, exchange.address(), &expected);
}
