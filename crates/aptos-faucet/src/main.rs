// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_sdk::types::{
    account_config::{testnet_dd_account_address, treasury_compliance_account_address},
    chain_id::ChainId,
    LocalAccount,
};
use std::sync::Arc;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Diem Faucet",
    author = "The Aptos Foundation",
    about = "Diem Testnet utitlty service for creating test account and minting test coins"
)]
struct Args {
    /// Faucet service listen address
    #[structopt(short = "a", long, default_value = "127.0.0.1")]
    pub address: String,
    /// Faucet service listen port
    #[structopt(short = "p", long, default_value = "80")]
    pub port: u16,
    /// Diem fullnode/validator server URL
    #[structopt(short = "s", long, default_value = "https://testnet.aptos-labs.com/")]
    pub server_url: String,
    /// Path to the private key for creating test account and minting coins.
    /// To keep Testnet simple, we used one private key for both treasury compliance account and testnet
    /// designated dealer account, hence here we only accept one private key.
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[structopt(short = "m", long, default_value = "/opt/diem/etc/mint.key")]
    pub mint_key_file_path: String,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: \"MAINNET\" or 1, testnet: \"TESTNET\" or 2, devnet: \"DEVNET\" or 3, \
    /// local swarm: \"TESTING\" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[structopt(short = "c", long, default_value = "2")]
    pub chain_id: ChainId,
}

#[tokio::main]
async fn main() {
    let args = Args::from_args();
    aptos_logger::Logger::new().init();

    let address: std::net::SocketAddr = format!("{}:{}", args.address, args.port)
        .parse()
        .expect("invalid address or port number");

    info!(
        "[faucet]: chain id: {}, server url: {}",
        args.chain_id,
        args.server_url.as_str(),
    );
    let treasury_account = LocalAccount::new(
        treasury_compliance_account_address(),
        generate_key::load_key(&args.mint_key_file_path),
        0,
    );
    let dd_account = LocalAccount::new(
        testnet_dd_account_address(),
        generate_key::load_key(&args.mint_key_file_path),
        0,
    );
    let service = Arc::new(aptos_faucet::Service::new(
        args.server_url,
        args.chain_id,
        treasury_account,
        dd_account,
    ));

    info!("[faucet]: running on: {}", address);
    warp::serve(aptos_faucet::routes(service))
        .run(address)
        .await;
}

#[cfg(test)]
mod tests {
    use aptos_crypto::hash::HashValue;
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
        transaction_builder::stdlib::{ScriptCall, ScriptFunctionCall},
        types::{
            account_address::AccountAddress,
            account_config::{testnet_dd_account_address, treasury_compliance_account_address},
            chain_id::ChainId,
            transaction::{
                authenticator::AuthenticationKey,
                metadata::{CoinTradeMetadata, Metadata},
                SignedTransaction, Transaction, TransactionPayload,
                TransactionPayload::{Script, ScriptFunction},
            },
            LocalAccount,
        },
    };
    use serde::Serialize;
    use std::{
        collections::HashMap,
        convert::TryFrom,
        str::FromStr,
        sync::{Arc, Mutex},
    };
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

    fn setup() -> (AccountStates, Arc<Service>) {
        let f = tempfile::NamedTempFile::new()
            .unwrap()
            .into_temp_path()
            .to_path_buf();
        generate_key::generate_and_save_key(&f);
        let treasury_account = LocalAccount::new(
            treasury_compliance_account_address(),
            generate_key::load_key(&f),
            0,
        );
        let dd_account =
            LocalAccount::new(testnet_dd_account_address(), generate_key::load_key(&f), 0);

        let chain_id = ChainId::test();

        let accounts = AccountStates::new(aptos_infallible::RwLock::new(HashMap::new()));
        accounts
            .write()
            .insert(testnet_dd_account_address(), AccountState::new(100000));
        accounts.write().insert(
            treasury_compliance_account_address(),
            AccountState::new(100000),
        );

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
            treasury_account,
            dd_account,
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
            match ScriptCall::decode(script) {
                Some(ScriptCall::CreateParentVaspAccount {
                    new_account_address: address,
                    ..
                }) => {
                    let mut writer = accounts.write();
                    let previous = writer.insert(address, AccountState::new(0));
                    assert!(previous.is_none(), "should not create account twice");
                }
                Some(ScriptCall::CreateDesignatedDealer { addr: address, .. }) => {
                    let mut writer = accounts.write();
                    let previous = writer.insert(address, AccountState::new(0));
                    assert!(previous.is_none(), "should not create account twice");
                }
                Some(ScriptCall::PeerToPeerWithMetadata { payee, amount, .. }) => {
                    let mut writer = accounts.write();
                    let account = writer.get_mut(&payee).expect("account should be created");
                    account.balance = amount;
                }
                _ => panic!("unexpected type of script"),
            }
        }
        if let Some(script_function) = ScriptFunctionCall::decode(txn.payload()) {
            match script_function {
                ScriptFunctionCall::AddVaspDomain { .. } => {}
                ScriptFunctionCall::RemoveVaspDomain { .. } => {}
                ScriptFunctionCall::CreateParentVaspAccount {
                    new_account_address: address,
                    ..
                } => {
                    let mut writer = accounts.write();
                    let previous = writer.insert(address, AccountState::new(0));
                    assert!(previous.is_none(), "should not create account twice");
                }
                ScriptFunctionCall::CreateDesignatedDealer { addr: address, .. } => {
                    let mut writer = accounts.write();
                    let previous = writer.insert(address, AccountState::new(0));
                    assert!(previous.is_none(), "should not create account twice");
                }
                ScriptFunctionCall::PeerToPeerWithMetadata { payee, amount, .. } => {
                    let mut writer = accounts.write();
                    let account = writer.get_mut(&payee).expect("account should be created");
                    account.balance = amount;
                }
                script => panic!("unexpected type of script: {:?}", script),
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
    async fn test_healthy() {
        let (_accounts, service) = setup();
        let filter = routes(service);
        let resp = warp::test::request()
            .method("GET")
            .path("/-/healthy")
            .reply(&filter)
            .await;
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), "aptos-faucet:ok");
    }

    #[tokio::test]
    async fn test_mint() {
        let (accounts, service) = setup();
        let filter = routes(service);

        // auth_key is outside of the loop for minting same account multiple
        // times, it should success and should not create same account multiple
        // times.
        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        for (i, path) in ["/", "/mint"].iter().enumerate() {
            let resp = warp::test::request()
                .method("POST")
                .path(
                    format!(
                        "{}?auth_key={}&amount={}&currency_code=XDX",
                        path, auth_key, amount
                    )
                    .as_str(),
                )
                .reply(&filter)
                .await;
            assert_eq!(resp.body(), (i + 1).to_string().as_str());
            let reader = accounts.read();
            let addr =
                AccountAddress::try_from("a74fd7c46952c497e75afb0a7932586d".to_owned()).unwrap();
            let account = reader.get(&addr).expect("account should be created");
            assert_eq!(account.balance, amount);
        }
    }

    #[tokio::test]
    async fn test_mint_with_txns_response() {
        let (accounts, service) = setup();
        let filter = routes(service);

        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let trade_id = "11111111-1111-1111-1111-111111111111";
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?auth_key={}&amount={}&trade_id={}&currency_code=XDX&return_txns=true",
                    auth_key, amount, trade_id
                )
                .as_str(),
            )
            .reply(&filter)
            .await;
        let body = resp.body();
        let txns: Vec<SignedTransaction> =
            bcs::from_bytes(&hex::decode(body).expect("hex encoded response body"))
                .expect("valid bcs vec");
        assert_eq!(txns.len(), 2);

        let trade_ids = get_trade_ids_from_payload(txns[1].payload());
        assert_eq!(trade_ids.len(), 1);
        assert_eq!(trade_ids[0], trade_id);

        let reader = accounts.read();
        let addr = AccountAddress::try_from("a74fd7c46952c497e75afb0a7932586d".to_owned()).unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_dd_account_with_txns_response() {
        let (accounts, service) = setup();
        let filter = routes(service);

        let auth_key = "44b8f03f203ec45dbd7484e433752efe54aa533116e934f8a50c28bece06d3ac";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?auth_key={}&amount={}&currency_code=XDX&return_txns=true&is_designated_dealer=true",
                    auth_key, amount
                )
                    .as_str(),
            )
            .reply(&filter)
            .await;
        let body = resp.body();
        let txns: Vec<SignedTransaction> =
            bcs::from_bytes(&hex::decode(body).expect("hex encoded response body"))
                .expect("valid bcs vec");
        assert_eq!(txns.len(), 2);

        let reader = accounts.read();
        let addr = AccountAddress::try_from("54aa533116e934f8a50c28bece06d3ac".to_owned()).unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_invalid_auth_key() {
        let (_accounts, service) = setup();
        let filter = routes(service);

        let auth_key = "invalid-auth-key";
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?auth_key={}&amount=1000000&currency_code=XDX",
                    auth_key
                )
                .as_str(),
            )
            .reply(&filter)
            .await;
        assert_eq!(resp.body(), "Invalid query string");
    }

    #[tokio::test]
    async fn test_mint_fullnode_error() {
        let (accounts, service) = setup();
        accounts
            .write()
            .remove(&treasury_compliance_account_address());
        let filter = routes(service);

        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?auth_key={}&amount=1000000&currency_code=XDX",
                    auth_key
                )
                .as_str(),
            )
            .reply(&filter)
            .await;
        assert_eq!(resp.body(), "treasury compliance account not found");
    }

    #[tokio::test]
    async fn create_account_with_client() {
        let (_accounts, service) = setup();
        let endpoint = service.endpoint().to_owned();
        let (address, future) = warp::serve(routes(service)).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let faucet_client = FaucetClient::new(format!("http://{}", address), endpoint);

        let auth_key = AuthenticationKey::from_str(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d",
        )
        .unwrap();
        tokio::task::spawn_blocking(move || faucet_client.create_account(auth_key, "XUS").unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn fund_account_with_client() {
        let (_accounts, service) = setup();
        let endpoint = service.endpoint().to_owned();
        let (address, future) = warp::serve(routes(service)).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let faucet_client = FaucetClient::new(format!("http://{}", address), endpoint);

        let auth_key = AuthenticationKey::from_str(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d",
        )
        .unwrap();
        tokio::task::spawn_blocking(move || {
            faucet_client.create_account(auth_key, "XUS").unwrap();
            faucet_client
                .fund(auth_key.derived_address(), "XUS", 10)
                .unwrap()
        })
        .await
        .unwrap();
    }

    fn get_trade_ids_from_payload(payload: &TransactionPayload) -> Vec<String> {
        match payload {
            Script(script) => match ScriptCall::decode(script) {
                Some(ScriptCall::PeerToPeerWithMetadata { metadata, .. }) => {
                    match bcs::from_bytes(&metadata).expect("should decode metadata") {
                        Metadata::CoinTradeMetadata(CoinTradeMetadata::CoinTradeMetadataV0(
                            coin_trade_metadata,
                        )) => coin_trade_metadata.trade_ids,
                        _ => panic!("unexpected type of transaction metadata"),
                    }
                }
                _ => panic!("unexpected type of script"),
            },
            ScriptFunction(_) => match ScriptFunctionCall::decode(payload) {
                Some(ScriptFunctionCall::PeerToPeerWithMetadata { metadata, .. }) => {
                    match bcs::from_bytes(&metadata).expect("should decode metadata") {
                        Metadata::CoinTradeMetadata(CoinTradeMetadata::CoinTradeMetadataV0(
                            coin_trade_metadata,
                        )) => coin_trade_metadata.trade_ids,
                        _ => panic!("unexpected type of transaction metadata"),
                    }
                }
                _ => panic!("unexpected type of script"),
            },

            _ => panic!("unexpected payload type"),
        }
    }
}
