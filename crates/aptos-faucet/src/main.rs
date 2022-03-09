// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_sdk::types::{
    account_address::AccountAddress, account_config::aptos_root_address, chain_id::ChainId,
    LocalAccount,
};
use std::sync::Arc;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Aptos Faucet",
    author = "The Aptos Foundation",
    about = "Aptos Testnet utility service for creating test accounts and minting test coins"
)]
struct Args {
    /// Faucet service listen address
    #[structopt(short = "a", long, default_value = "127.0.0.1")]
    pub address: String,
    /// Faucet service listen port
    #[structopt(short = "p", long, default_value = "80")]
    pub port: u16,
    /// Aptos fullnode/validator server URL
    #[structopt(short = "s", long, default_value = "https://testnet.aptoslabs.com/")]
    pub server_url: String,
    /// Path to the private key for creating test account and minting coins.
    /// To keep Testnet simple, we used one private key for aptos root account
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[structopt(short = "m", long, default_value = "/opt/aptos/etc/mint.key")]
    pub mint_key_file_path: String,
    /// Address of the account to send transactions from.
    /// On Testnet, for example, this is 0xa550c18.
    /// If not present, the mint key's address is used
    #[structopt(short = "t", long, parse(try_from_str = AccountAddress::from_hex_literal))]
    pub mint_account_address: Option<AccountAddress>,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: "MAINNET" or 1, testnet: "TESTNET" or 2, devnet: "DEVNET" or 3,
    /// local swarm: "TESTING" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[structopt(short = "c", long, default_value = "2")]
    pub chain_id: ChainId,
    /// Fixed amount of coins to mint.
    /// If this is unset, users can specify the amount to mint.
    /// For Aptos public testnets, this is always set.
    #[structopt(short = "f", long)]
    pub fixed_amount: Option<u64>,
}

#[tokio::main]
async fn main() {
    let args = Args::from_args();
    aptos_logger::Logger::new().init();

    let address: std::net::SocketAddr = format!("{}:{}", args.address, args.port)
        .parse()
        .expect("invalid address or port number");

    info!(
        "[faucet]: chain id: {}, server url: {} . Limit: {:?}",
        args.chain_id,
        args.server_url.as_str(),
        args.fixed_amount,
    );

    let key = generate_key::load_key(&args.mint_key_file_path);

    let faucet_address: AccountAddress =
        args.mint_account_address.unwrap_or_else(aptos_root_address);
    let faucet_account = LocalAccount::new(faucet_address, key, 0);
    let service = Arc::new(aptos_faucet::Service::new(
        args.server_url,
        args.chain_id,
        faucet_account,
        args.fixed_amount,
    ));

    info!(
        "[faucet]: running on: {}. Minting from {}",
        address, faucet_address
    );
    warp::serve(aptos_faucet::routes(service))
        .run(address)
        .await;
}

#[cfg(test)]
mod tests {
    use aptos_crypto::{ed25519::Ed25519PublicKey, hash::HashValue, PrivateKey};
    use aptos_faucet::{routes, Service};
    use aptos_infallible::RwLock;
    use aptos_rest_client::{
        aptos_api_types::{
            AccountData, DirectWriteSet, LedgerInfo, PendingTransaction, Response,
            TransactionPayload as TransactionPayloadData, WriteSet, WriteSetPayload,
        },
        FaucetClient,
    };
    use aptos_sdk::{
        transaction_builder::aptos_stdlib::ScriptFunctionCall,
        types::{
            account_address::AccountAddress,
            chain_id::ChainId,
            transaction::{
                authenticator::AuthenticationKey, SignedTransaction, Transaction,
                TransactionPayload::Script,
            },
            LocalAccount,
        },
    };
    use serde::Serialize;
    use std::{
        collections::HashMap,
        convert::{TryFrom, TryInto},
        sync::{Arc, Mutex},
    };
    use tokio::task::yield_now;
    use warp::{Filter, Rejection, Reply};

    type AccountStates = Arc<RwLock<HashMap<AccountAddress, AccountState>>>;
    #[derive(Clone, Debug, Eq, PartialEq, Hash)]
    struct AccountState {
        pub authentication_key: AuthenticationKey,
        pub balance: u64,
        pub sequence_number: u64,
    }

    impl AccountState {
        pub fn new(balance: u64) -> Self {
            Self {
                authentication_key: AuthenticationKey::new([1; 32]),
                balance,
                sequence_number: 0,
            }
        }
    }

    fn setup(fixed_amount: Option<u64>) -> (AccountStates, Arc<Service>) {
        let f = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_path_buf();
        generate_key::generate_and_save_key(&f);
        let key = generate_key::load_key(&f);
        let account_address = AuthenticationKey::ed25519(&key.public_key()).derived_address();

        let faucet_account = LocalAccount::new(account_address, key, 0);

        let chain_id = ChainId::test();

        let accounts = AccountStates::new(aptos_infallible::RwLock::new(HashMap::new()));
        accounts
            .write()
            .insert(account_address, AccountState::new(0));

        let last_txn = Arc::new(Mutex::new(None));
        let last_txn_0 = last_txn.clone();

        let accounts_cloned_0 = accounts.clone();
        let accounts_cloned_1 = accounts.clone();
        let stub = warp::path!("accounts" / String)
            .and(warp::any().map(move || accounts_cloned_0.clone()))
            .and_then(handle_get_account)
            .or(warp::path!("transactions" / String)
                .and(warp::get())
                .and(warp::any().map(move || last_txn_0.clone()))
                .and_then(handle_get_transaction))
            .or(warp::path!("transactions")
                .and(warp::post())
                .and(warp::body::bytes())
                .and(warp::any().map(move || (accounts_cloned_1.clone(), last_txn.clone())))
                .and_then(handle_submit_transaction));
        let (address, future) = warp::serve(stub).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let service = Service::new(
            format!("http://localhost:{}/", address.port()),
            chain_id,
            faucet_account,
            fixed_amount,
        );
        (accounts, Arc::new(service))
    }

    async fn handle_get_account(
        address: String,
        accounts: AccountStates,
    ) -> Result<impl Reply, Rejection> {
        let reader = accounts.read();
        let account = AccountAddress::try_from(address)
            .ok()
            .and_then(|address| reader.get(&address));

        if let Some(account) = account {
            let auth_vec: Vec<u8> = account.authentication_key.as_ref().into();
            let account_data = AccountData {
                authentication_key: auth_vec.into(),
                sequence_number: account.sequence_number.into(),
            };
            Ok(response(&account_data))
        } else {
            Err(warp::reject())
        }
    }

    async fn handle_get_transaction(
        _hash: String,
        last_txn: Arc<Mutex<Option<Transaction>>>,
    ) -> Result<impl Reply, Rejection> {
        last_txn.lock().unwrap().as_ref().map_or_else(
            || Err(warp::reject()),
            |txn| {
                let info = aptos_rest_client::aptos_api_types::TransactionInfo {
                    version: 0.into(),
                    hash: HashValue::zero().into(),
                    state_root_hash: HashValue::zero().into(),
                    event_root_hash: HashValue::zero().into(),
                    gas_used: 0.into(),
                    success: true,
                    vm_status: "Executed".to_string(),
                    accumulator_root_hash: HashValue::zero().into(),
                };
                let serializable_txn: aptos_rest_client::aptos_api_types::Transaction = (
                    txn.as_signed_user_txn().unwrap(),
                    info,
                    dummy_payload(),
                    Vec::new(),
                    0,
                )
                    .into();

                Ok(response(&serializable_txn))
            },
        )
    }

    async fn handle_submit_transaction(
        txn: bytes::Bytes,
        (accounts, last_txn): (AccountStates, Arc<Mutex<Option<Transaction>>>),
    ) -> Result<impl Reply, Rejection> {
        let txn: SignedTransaction = bcs::from_bytes(&txn).unwrap();
        assert_eq!(txn.chain_id(), ChainId::test());

        if let Script(script) = txn.payload() {
            panic!("unexpected type of script: {:?}", script.args())
        }
        if let Some(script_function) = ScriptFunctionCall::decode(txn.payload()) {
            match script_function {
                ScriptFunctionCall::CreateAccount {
                    new_account_address: address,
                    ..
                } => {
                    let mut writer = accounts.write();
                    let previous = writer.insert(address, AccountState::new(0));
                    assert!(previous.is_none(), "should not create account twice");
                }
                ScriptFunctionCall::Mint { addr, amount, .. } => {
                    // Sometimes we call CreateAccount and Mint at the same time (from our tests: this is a test method)
                    // If the account doesn't exist yet, we sleep for 100ms to let the other request finish
                    if accounts.write().get_mut(&addr).is_none() {
                        yield_now().await;
                    }
                    let mut writer = accounts.write();
                    let account = writer.get_mut(&addr).expect("account should be created");
                    account.balance += amount;
                }
                script => panic!("unexpected type of script function: {:?}", script),
            }
        }

        let pending_txn = PendingTransaction {
            hash: HashValue::zero().into(),
            request: (&txn, dummy_payload()).into(),
        };

        *last_txn.lock().unwrap() = Some(Transaction::UserTransaction(txn));
        Ok(response(&pending_txn))
    }

    fn response<T: Serialize>(body: &T) -> warp::reply::Response {
        let li = LedgerInfo {
            chain_id: ChainId::test().id(),
            ledger_version: 5.into(),
            ledger_timestamp: 5.into(),
        };
        Response::new(li, body).unwrap().into_response()
    }

    fn dummy_payload() -> TransactionPayloadData {
        TransactionPayloadData::WriteSetPayload(WriteSetPayload {
            write_set: WriteSet::DirectWriteSet(DirectWriteSet {
                changes: Vec::new(),
                events: Vec::new(),
            }),
        })
    }

    #[tokio::test]
    async fn test_mint() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        // pub_key is outside of the loop for minting same account multiple times,
        // it should succeed and should not create same account multiple times.
        let pub_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?pub_key={}&amount={}", pub_key, amount).as_str())
            .reply(&filter)
            .await;
        assert_eq!(resp.body(), 2.to_string().as_str());
        let reader = accounts.read();
        let addr = AccountAddress::try_from("25C62C0E0820F422000814CDBA407835".to_owned()).unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_with_txns_response() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        let pub_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?pub_key={}&amount={}&return_txns=true",
                    pub_key, amount
                )
                .as_str(),
            )
            .reply(&filter)
            .await;
        let body = resp.body();
        let bytes = hex::decode(body).expect("hex encoded response body");
        let txns: Vec<SignedTransaction> = bcs::from_bytes(&bytes).expect("valid bcs vec");
        assert_eq!(txns.len(), 2);

        let reader = accounts.read();
        let addr = AccountAddress::try_from("25C62C0E0820F422000814CDBA407835".to_owned()).unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_health() {
        let (_accounts, service) = setup(None);

        let resp = warp::test::request()
            .method("GET")
            .path(&"/health".to_string())
            .reply(&routes(service))
            .await;

        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), 0.to_string().as_str());
    }

    #[tokio::test]
    async fn test_mint_send_amount_when_not_allowed() {
        let (_accounts, service) = setup(Some(7));
        let filter = routes(service);

        let pub_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?pub_key={}&amount=1000000", pub_key).as_str())
            .reply(&filter)
            .await;
        assert_eq!(resp.body(), "Mint amount is fixed to 7 on this faucet");
    }

    #[tokio::test]
    async fn test_mint_invalid_pub_key() {
        let (_accounts, service) = setup(None);
        let filter = routes(service);

        let pub_key = "invalid-auth-key";
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?pub_key={}&amount=1000000", pub_key).as_str())
            .reply(&filter)
            .await;
        assert_eq!(resp.body(), "Invalid query string");
    }

    #[tokio::test]
    async fn test_mint_fullnode_error() {
        let (accounts, service) = setup(None);
        let address = service.faucet_account.lock().unwrap().address();
        accounts.write().remove(&address);
        let filter = routes(service);

        let pub_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?pub_key={}&amount=1000000", pub_key).as_str())
            .reply(&filter)
            .await;

        assert_eq!(
            resp.body(),
            &format!("faucet account {:?} not found", address)
        );
    }

    #[tokio::test]
    async fn create_account_with_client() {
        let (_accounts, service) = setup(None);
        let endpoint = service.endpoint().to_owned();
        let (address, future) = warp::serve(routes(service)).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let faucet_client = FaucetClient::new(format!("http://{}", address), endpoint);

        let pub_key: Ed25519PublicKey =
            hex::decode(&"459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d")
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap();

        assert_eq!(
            AuthenticationKey::ed25519(&pub_key)
                .derived_address()
                .to_string(),
            "25C62C0E0820F422000814CDBA407835"
        );

        let res = tokio::task::spawn_blocking(move || faucet_client.create_account(pub_key))
            .await
            .unwrap();
        res.unwrap();
    }

    #[tokio::test]
    async fn fund_account_with_client() {
        let (_accounts, service) = setup(None);
        let endpoint = service.endpoint().to_owned();
        let (address, future) = warp::serve(routes(service)).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let faucet_client = FaucetClient::new(format!("http://{}", address), endpoint);

        let pub_key: Ed25519PublicKey =
            hex::decode(&"459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d")
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap();

        let (res1, res2) = tokio::task::spawn_blocking(move || {
            let address = AuthenticationKey::ed25519(&pub_key).derived_address();
            (
                faucet_client.create_account(pub_key),
                faucet_client.fund(address, 10),
            )
        })
        .await
        .unwrap();

        res1.unwrap();
        res2.unwrap();
    }
}
