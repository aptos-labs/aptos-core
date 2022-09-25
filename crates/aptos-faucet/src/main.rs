// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_faucet::FaucetArgs;
use clap::Parser;

#[tokio::main]
async fn main() {
    aptos_logger::Logger::new().init();
    let args: FaucetArgs = FaucetArgs::from_args();
    args.run().await
}

#[cfg(test)]
mod tests {
    use aptos_crypto::{ed25519::Ed25519PublicKey, hash::HashValue};
    use aptos_faucet::{routes, Service};
    use aptos_infallible::RwLock;
    use aptos_keygen::KeyGen;
    use aptos_rest_client::{
        aptos_api_types::{
            AccountData, LedgerInfo, ModuleBundlePayload, PendingTransaction,
            TransactionPayload as TransactionPayloadData,
        },
        FaucetClient,
    };
    use aptos_sdk::types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            authenticator::AuthenticationKey, SignedTransaction, Transaction, TransactionArgument,
            TransactionPayload::Script,
        },
        LocalAccount,
    };
    use aptos_warp_webserver::Response;
    use serde::Serialize;
    use std::{
        collections::HashMap,
        convert::{Infallible, TryFrom, TryInto},
        sync::{Arc, Mutex},
    };
    use tokio::task::JoinHandle;
    use url::Url;
    use warp::{
        body::BodyDeserializeError,
        cors::CorsForbidden,
        http::{header, HeaderValue, StatusCode},
        reject::{LengthRequired, MethodNotAllowed, PayloadTooLarge, UnsupportedMediaType},
        reply, Filter, Rejection, Reply,
    };

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

    fn setup(maximum_amount: Option<u64>) -> (AccountStates, Arc<Service>) {
        let mut keygen = KeyGen::from_seed([0; 32]);
        let (private_key, public_key) = keygen.generate_ed25519_keypair();
        let account_address = AuthenticationKey::ed25519(&public_key).derived_address();

        let faucet_account = LocalAccount::new(account_address, private_key, 0);

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
            .or(warp::path!("transactions" / "by_hash" / String)
                .and(warp::get())
                .and(warp::any().map(move || last_txn_0.clone()))
                .and_then(handle_get_transaction))
            .or(warp::path!("transactions")
                .and(warp::post())
                .and(warp::body::bytes())
                .and(warp::any().map(move || (accounts_cloned_1.clone(), last_txn.clone())))
                .and_then(handle_submit_transaction))
            .with(
                warp::cors()
                    .allow_any_origin()
                    .allow_methods(vec!["POST", "GET"])
                    .allow_headers(vec![header::CONTENT_TYPE]),
            )
            .recover(handle_rejection);
        let (address, future) = warp::serve(stub).bind_ephemeral(([127, 0, 0, 1], 0));
        tokio::task::spawn(async move { future.await });

        let service = Service::new(
            Url::parse(&format!("http://localhost:{}/", address.port())).unwrap(),
            chain_id,
            faucet_account,
            maximum_amount,
        )
        .configure_for_testing();
        (accounts, Arc::new(service))
    }

    async fn handle_get_account(
        address: String,
        accounts: AccountStates,
    ) -> Result<impl Reply, Rejection> {
        let reader = accounts.read();
        let account = match AccountAddress::try_from(address.clone())
            .or_else(|_e| AccountAddress::from_hex(address.clone()))
        {
            Ok(addr) => reader.get(&addr),
            _ => None,
        };
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
                    state_change_hash: HashValue::zero().into(),
                    event_root_hash: HashValue::zero().into(),
                    state_checkpoint_hash: None,
                    gas_used: 0.into(),
                    success: true,
                    vm_status: "Executed".to_string(),
                    accumulator_root_hash: HashValue::zero().into(),
                    changes: vec![],
                    block_height: None,
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
            let dst_addr = if let TransactionArgument::Address(addr) = script.args()[0] {
                addr
            } else {
                panic!("unexpected type of script: {:?}", script);
            };
            let amount = if let TransactionArgument::U64(amount) = script.args()[1] {
                amount
            } else {
                panic!("unexpected type of script: {:?}", script);
            };

            accounts
                .write()
                .entry(dst_addr)
                .and_modify(|account| account.balance += amount)
                .or_insert_with(|| AccountState::new(amount));
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
            epoch: 1.into(),
            ledger_version: 5.into(),
            oldest_ledger_version: 0.into(),
            block_height: 4.into(),
            oldest_block_height: 0.into(),
            ledger_timestamp: 5.into(),
        };
        Response::new(li, body).unwrap().into_response()
    }

    fn dummy_payload() -> TransactionPayloadData {
        TransactionPayloadData::ModuleBundlePayload(ModuleBundlePayload { modules: vec![] })
    }

    #[derive(Clone, Debug, Serialize, PartialEq, Eq)]
    pub struct Error {
        pub code: u16,
        pub message: String,
    }

    impl Error {
        fn new(code: StatusCode, message: String) -> Error {
            Error {
                code: code.as_u16(),
                message,
            }
        }

        fn status_code(&self) -> StatusCode {
            StatusCode::from_u16(self.code).unwrap()
        }
    }

    async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
        let code;
        let body;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            body = reply::json(&Error::new(code, "Not Found".to_owned()));
        } else if let Some(error) = err.find::<Error>() {
            code = error.status_code();
            body = reply::json(error);
        } else if let Some(cause) = err.find::<CorsForbidden>() {
            code = StatusCode::FORBIDDEN;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else if let Some(cause) = err.find::<BodyDeserializeError>() {
            code = StatusCode::BAD_REQUEST;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else if let Some(cause) = err.find::<LengthRequired>() {
            code = StatusCode::LENGTH_REQUIRED;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else if let Some(cause) = err.find::<PayloadTooLarge>() {
            code = StatusCode::PAYLOAD_TOO_LARGE;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else if let Some(cause) = err.find::<UnsupportedMediaType>() {
            code = StatusCode::UNSUPPORTED_MEDIA_TYPE;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else if let Some(cause) = err.find::<MethodNotAllowed>() {
            code = StatusCode::METHOD_NOT_ALLOWED;
            body = reply::json(&Error::new(code, cause.to_string()));
        } else {
            code = StatusCode::INTERNAL_SERVER_ERROR;
            body = reply::json(&Error::new(code, format!("unexpected error: {:?}", err)));
        }
        let mut rep = reply::with_status(body, code).into_response();
        rep.headers_mut()
            .insert("access-control-allow-origin", HeaderValue::from_static("*"));
        Ok(rep)
    }

    #[tokio::test]
    async fn test_mint_auth_key() {
        let (accounts, service) = setup(None);
        let filter = routes(service);
        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?auth_key={}&amount={}", auth_key, amount).as_str())
            .reply(&filter)
            .await;
        serde_json::from_slice::<Vec<HashValue>>(resp.body()).unwrap();
        let reader = accounts.read();
        let addr = AccountAddress::try_from(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d".to_owned(),
        )
        .unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_pub_key() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        let pub_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?pub_key={}&amount={}", pub_key, amount).as_str())
            .reply(&filter)
            .await;
        serde_json::from_slice::<Vec<HashValue>>(resp.body()).unwrap();
        let reader = accounts.read();
        let addr = AccountAddress::try_from(
            "9FF98E82355EB13098F3B1157AC018A725C62C0E0820F422000814CDBA407835".to_owned(),
        )
        .unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_address() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        let address = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?address={}&amount={}", address, amount).as_str())
            .reply(&filter)
            .await;

        serde_json::from_slice::<Vec<HashValue>>(resp.body()).unwrap();
        let reader = accounts.read();
        let addr = AccountAddress::try_from(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d".to_owned(),
        )
        .unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_address_hex() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        let address = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?address={}&amount={}", address, amount).as_str())
            .reply(&filter)
            .await;

        serde_json::from_slice::<Vec<HashValue>>(resp.body()).unwrap();
        let reader = accounts.read();
        let addr = AccountAddress::try_from(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d".to_owned(),
        )
        .unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_mint_with_txns_response() {
        let (accounts, service) = setup(None);
        let filter = routes(service);

        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let amount = 13345;
        let resp = warp::test::request()
            .method("POST")
            .path(
                format!(
                    "/mint?auth_key={}&amount={}&return_txns=true",
                    auth_key, amount
                )
                .as_str(),
            )
            .reply(&filter)
            .await;
        let body = resp.body();
        let bytes = hex::decode(body).expect("hex encoded response body");
        bcs::from_bytes::<Vec<SignedTransaction>>(&bytes).expect("valid bcs vec");

        let reader = accounts.read();
        let addr = AccountAddress::try_from(
            "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d".to_owned(),
        )
        .unwrap();
        let account = reader.get(&addr).expect("account should be created");
        assert_eq!(account.balance, amount);
    }

    #[tokio::test]
    async fn test_health() {
        let (_accounts, service) = setup(None);

        let resp = warp::test::request()
            .method("GET")
            .path("/health")
            .reply(&routes(service))
            .await;

        assert_eq!(resp.status(), 200);
        assert_eq!(resp.body(), std::string::ToString::to_string(&0).as_str());
    }

    #[tokio::test]
    async fn test_mint_invalid_auth_key() {
        let (_accounts, service) = setup(None);
        let filter = routes(service);

        let auth_key = "invalid-auth-key";
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?auth_key={}&amount=1000000", auth_key).as_str())
            .reply(&filter)
            .await;
        assert_eq!(
            resp.body(),
            "You must provide 'address' (preferred), 'pub_key', or 'auth_key'"
        );
    }

    #[tokio::test]
    async fn test_mint_fullnode_error() {
        let (accounts, service) = setup(None);
        let address = service.faucet_account.lock().await.address();
        accounts.write().remove(&address);
        let filter = routes(service);

        let auth_key = "459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d";
        let resp = warp::test::request()
            .method("POST")
            .path(format!("/mint?auth_key={}&amount=1000000", auth_key).as_str())
            .reply(&filter)
            .await;

        assert!(
            resp.body().starts_with(
                format!(
                    "Faucet account {:?} not found: HTTP error 404 Not Found:",
                    address
                )
                .as_str()
                .as_bytes()
            ),
            "{} did not start with the expected string",
            std::str::from_utf8(resp.body()).unwrap()
        );
    }

    #[tokio::test]
    async fn create_account_with_client() {
        let (faucet_client, _service) = get_client().await;
        let address = get_address();
        faucet_client.create_account(address).await.unwrap();
    }

    #[tokio::test]
    async fn fund_account_with_client() {
        let (faucet_client, _service) = get_client().await;
        let address = get_address();
        faucet_client.create_account(address).await.unwrap();
        faucet_client.fund(address, 10).await.unwrap();
    }

    async fn get_client() -> (FaucetClient, JoinHandle<()>) {
        let (_accounts, service) = setup(None);
        let endpoint = service.endpoint().clone();
        let (address, future) = warp::serve(routes(service)).bind_ephemeral(([127, 0, 0, 1], 0));
        let service = tokio::task::spawn(async move { future.await });

        let faucet_client = FaucetClient::new_for_testing(
            Url::parse(&format!("http://{}", address)).unwrap(),
            endpoint,
        );

        (faucet_client, service)
    }

    fn get_address() -> AccountAddress {
        let pub_key: Ed25519PublicKey =
            hex::decode(&"459c77a38803bd53f3adee52703810e3a74fd7c46952c497e75afb0a7932586d")
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap();
        let address = AuthenticationKey::ed25519(&pub_key).derived_address();
        assert_eq!(
            address.to_string(),
            "9ff98e82355eb13098f3b1157ac018a725c62c0e0820f422000814cdba407835"
        );
        address
    }
}
