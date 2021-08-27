// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_config::config::{Identity, NodeConfig, SecureBackend};
use diem_crypto::ed25519::Ed25519PublicKey;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::{Currency, TransactionFactory},
    types::{transaction::SignedTransaction, LocalAccount},
};
use forge::{LocalSwarm, Swarm};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::{collections::BTreeMap, fs::File, io::Write, path::PathBuf, str::FromStr};

// TODO(joshlind): Refactor all of these so that they can be contained within the calling
// test files and not shared across all tests.
pub fn compare_balances(
    expected_balances: Vec<(f64, String)>,
    extracted_balances: Vec<String>,
) -> bool {
    if extracted_balances.len() != extracted_balances.len() {
        return false;
    }

    let extracted_balances_dec: BTreeMap<_, _> = extracted_balances
        .into_iter()
        .map(|balance_str| {
            let (currency_code, stripped_str) = if balance_str.ends_with("XUS") {
                ("XUS", balance_str.trim_end_matches("XUS"))
            } else if balance_str.ends_with("XDX") {
                ("XDX", balance_str.trim_end_matches("XDX"))
            } else {
                panic!("Unexpected currency type returned for balance")
            };
            (currency_code, Decimal::from_str(stripped_str).ok())
        })
        .collect();

    expected_balances
        .into_iter()
        .all(|(balance, currency_code)| {
            if let Some(extracted_balance) = extracted_balances_dec.get(currency_code.as_str()) {
                Decimal::from_f64(balance) == *extracted_balance
            } else {
                false
            }
        })
}

pub fn create_and_fund_account(swarm: &mut LocalSwarm, amount: u64) -> LocalAccount {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    swarm
        .chain_info()
        .create_parent_vasp_account(Currency::XUS, account.authentication_key())
        .unwrap();
    swarm
        .chain_info()
        .fund(Currency::XUS, account.address(), amount)
        .unwrap();
    account
}

pub fn transfer_coins_non_blocking(
    client: &BlockingClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn = sender.sign_with_transaction_builder(transaction_factory.peer_to_peer(
        Currency::XUS,
        receiver.address(),
        amount,
    ));

    client.submit(&txn).unwrap();
    txn
}

pub fn transfer_coins(
    client: &BlockingClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn = transfer_coins_non_blocking(client, transaction_factory, sender, receiver, amount);

    client
        .wait_for_signed_transaction(&txn, None, None)
        .unwrap();

    txn
}

pub fn assert_balance(client: &BlockingClient, account: &LocalAccount, balance: u64) {
    let account_view = client
        .get_account(account.address())
        .unwrap()
        .into_inner()
        .unwrap();

    let onchain_balance = account_view
        .balances
        .into_iter()
        .find(|amount_view| amount_view.currency == Currency::XUS)
        .unwrap();
    assert_eq!(onchain_balance.amount, balance);
}

/// This module provides useful functions for operating, handling and managing
/// DiemSwarm instances. It is particularly useful for working with tests that
/// require a SmokeTestEnvironment, as it provides a generic interface across
/// DiemSwarms, regardless of if the swarm is a validator swarm, validator full
/// node swarm, or a public full node swarm.
pub mod diem_swarm_utils {
    use crate::test_utils::fetch_backend_storage;
    use cli::client_proxy::ClientProxy;
    use diem_config::config::{NodeConfig, OnDiskStorageConfig, SecureBackend, WaypointConfig};
    use diem_global_constants::{DIEM_ROOT_KEY, TREASURY_COMPLIANCE_KEY};
    use diem_secure_storage::{CryptoStorage, KVStorage, OnDiskStorage, Storage};
    use diem_swarm::swarm::DiemSwarm;
    use diem_types::{chain_id::ChainId, waypoint::Waypoint};
    use forge::{LocalNode, LocalSwarm, Swarm};
    use std::path::PathBuf;

    /// Returns a new client proxy connected to the given swarm at the specified
    /// node index.
    pub fn get_client_proxy(
        swarm: &DiemSwarm,
        node_index: usize,
        diem_root_key_path: &str,
        mnemonic_file_path: PathBuf,
        waypoint: Option<Waypoint>,
    ) -> ClientProxy {
        let port = swarm.get_client_port(node_index);

        let mnemonic_file_path = mnemonic_file_path
            .canonicalize()
            .expect("Unable to get canonical path of mnemonic_file_path")
            .to_str()
            .unwrap()
            .to_string();

        ClientProxy::new(
            ChainId::test(),
            &format!("http://localhost:{}/v1", port),
            diem_root_key_path,
            diem_root_key_path,
            diem_root_key_path,
            false,
            /* faucet server */ None,
            Some(mnemonic_file_path),
            waypoint.unwrap_or(swarm.config.waypoint),
            true,
        )
        .unwrap()
    }

    /// Loads the nodes's storage backend identified by the node index in the given swarm.
    pub fn load_validators_backend_storage(validator: &LocalNode) -> SecureBackend {
        fetch_backend_storage(validator.config(), None)
    }

    pub fn create_root_storage(swarm: &mut LocalSwarm) -> SecureBackend {
        let chain_info = swarm.chain_info();
        let root_key =
            bcs::from_bytes(&bcs::to_bytes(chain_info.root_account.private_key()).unwrap())
                .unwrap();
        let treasury_compliance_key = bcs::from_bytes(
            &bcs::to_bytes(chain_info.treasury_compliance_account.private_key()).unwrap(),
        )
        .unwrap();

        let mut root_storage_config = OnDiskStorageConfig::default();
        root_storage_config.path = swarm.dir().join("root-storage.json");
        let mut root_storage = OnDiskStorage::new(root_storage_config.path());
        root_storage
            .import_private_key(DIEM_ROOT_KEY, root_key)
            .unwrap();
        root_storage
            .import_private_key(TREASURY_COMPLIANCE_KEY, treasury_compliance_key)
            .unwrap();

        SecureBackend::OnDiskStorage(root_storage_config)
    }

    pub fn insert_waypoint(node_config: &mut NodeConfig, waypoint: Waypoint) {
        let f = |backend: &SecureBackend| {
            let mut storage: Storage = backend.into();
            storage
                .set(diem_global_constants::WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
            storage
                .set(diem_global_constants::GENESIS_WAYPOINT, waypoint)
                .expect("Unable to write waypoint");
        };
        let backend = &node_config.consensus.safety_rules.backend;
        f(backend);
        match &node_config.base.waypoint {
            WaypointConfig::FromStorage(backend) => {
                f(backend);
            }
            _ => panic!("unexpected waypoint from node config"),
        }
    }
}

/// Loads the node's storage backend from the given node config. If a namespace
/// is specified, the storage namespace will be overridden.
fn fetch_backend_storage(
    node_config: &NodeConfig,
    overriding_namespace: Option<String>,
) -> SecureBackend {
    if let Identity::FromStorage(storage_identity) =
        &node_config.validator_network.as_ref().unwrap().identity
    {
        match storage_identity.backend.clone() {
            SecureBackend::OnDiskStorage(mut config) => {
                if let Some(namespace) = overriding_namespace {
                    config.namespace = Some(namespace);
                }
                SecureBackend::OnDiskStorage(config)
            }
            _ => unimplemented!("On-disk storage is the only backend supported in smoke tests"),
        }
    } else {
        panic!("Couldn't load identity from storage");
    }
}

/// Writes a given public key to a file specified by the given path using hex encoding.
/// Contents are written using utf-8 encoding and a newline is appended to ensure that
/// whitespace can be handled by tests.
pub fn write_key_to_file_hex_format(key: &Ed25519PublicKey, key_file_path: PathBuf) {
    let hex_encoded_key = hex::encode(key.to_bytes());
    let key_and_newline = hex_encoded_key + "\n";
    let mut file = File::create(key_file_path).unwrap();
    file.write_all(key_and_newline.as_bytes()).unwrap();
}

/// Writes a given public key to a file specified by the given path using bcs encoding.
pub fn write_key_to_file_bcs_format(key: &Ed25519PublicKey, key_file_path: PathBuf) {
    let bcs_encoded_key = bcs::to_bytes(&key).unwrap();
    let mut file = File::create(key_file_path).unwrap();
    file.write_all(&bcs_encoded_key).unwrap();
}
