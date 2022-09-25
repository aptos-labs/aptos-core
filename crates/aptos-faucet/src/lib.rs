// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This crate provides the Faucet service for creating and funding accounts on the Aptos Network.
//!
//! THIS SERVICE SHOULD NEVER BE DEPLOYED TO MAINNET.
//!
//! ## Launch service
//!
//! Launch faucet service locally and connect to Testnet:
//!
//! ```bash
//! cargo run --bin aptos-faucet -- -c TESTNET -m <mint-private-key-path> -s http://localhost:8080 -p 8081
//! ```
//!
//! Check help doc for options details:
//!
//! ```bash
//! cargo run -p aptos-faucet -- -h
//! ```

use anyhow::Result;
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress, account_config::aptos_test_root_address,
        chain_id::ChainId, LocalAccount,
    },
};
use clap::Parser;
use futures::lock::Mutex;
use reqwest::StatusCode;
use std::{convert::Infallible, path::PathBuf, sync::Arc};
use url::Url;
use warp::{http, Filter, Rejection, Reply};

pub mod mint;

/// Aptos Testnet utility service for creating test accounts and minting test coins
#[derive(Clone, Debug, Parser)]
#[clap(name = "Aptos Faucet", author, version)]
pub struct FaucetArgs {
    /// Faucet service listen address
    #[clap(short = 'a', long, default_value = "127.0.0.1")]
    pub address: String,
    /// Faucet service listen port
    #[clap(short = 'p', long, default_value = "80")]
    pub port: u16,
    /// Aptos fullnode/validator server URL
    #[clap(short = 's', long, default_value = "https://testnet.aptoslabs.com/")]
    pub server_url: Url,
    /// Path to the private key for creating test account and minting coins.
    /// To keep Testnet simple, we used one private key for aptos root account
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[clap(
        short = 'm',
        long,
        default_value = "/opt/aptos/etc/mint.key",
        parse(from_os_str)
    )]
    pub mint_key_file_path: PathBuf,
    /// Ed25519PrivateKey for minting coins
    #[clap(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,
    /// Address of the account to send transactions from.
    /// On Testnet, for example, this is a550c18.
    /// If not present, the mint key's address is used
    #[clap(short = 't', long, parse(try_from_str = AccountAddress::from_hex_literal))]
    pub mint_account_address: Option<AccountAddress>,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: "MAINNET" or 1, testnet: "TESTNET" or 2, devnet: "DEVNET" or 3,
    /// local swarm: "TESTING" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[clap(short = 'c', long, default_value = "2")]
    pub chain_id: ChainId,
    /// Maximum amount of coins to mint.
    #[clap(long)]
    pub maximum_amount: Option<u64>,
    #[clap(long)]
    pub do_not_delegate: bool,
}

impl FaucetArgs {
    pub async fn run(self) {
        let address: std::net::SocketAddr = format!("{}:{}", self.address, self.port)
            .parse()
            .expect("invalid address or port number");

        info!(
            "[faucet]: chain id: {}, server url: {} . Limit: {:?}",
            self.chain_id,
            self.server_url.as_str(),
            self.maximum_amount,
        );

        let key = if let Some(ref key) = self.mint_key {
            key.private_key()
        } else {
            bcs::from_bytes(
                &std::fs::read(self.mint_key_file_path.as_path())
                    .expect("Failed to read mint key file"),
            )
            .expect("Failed to deserialize mint key file")
        };

        let faucet_address: AccountAddress = self
            .mint_account_address
            .unwrap_or_else(aptos_test_root_address);
        let faucet_account = LocalAccount::new(faucet_address, key, 0);

        // Do not use maximum amount on delegation, this allows the new delegated faucet to
        // mint a lot for themselves!
        let maximum_amount = if self.do_not_delegate {
            self.maximum_amount
        } else {
            None
        };

        let service = Arc::new(Service::new(
            self.server_url.clone(),
            self.chain_id,
            faucet_account,
            maximum_amount,
        ));

        let actual_service = if self.do_not_delegate {
            service
        } else {
            delegate_mint_account(service, self.server_url, self.chain_id, self.maximum_amount)
                .await
        };

        println!("Faucet is running.  Faucet endpoint: {}", address);

        info!(
            "[faucet]: running on: {}. Minting from {}",
            address,
            actual_service.faucet_account.lock().await.address()
        );
        warp::serve(routes(actual_service)).run(address).await;
    }
}

pub struct Service {
    pub faucet_account: Mutex<LocalAccount>,
    pub transaction_factory: TransactionFactory,
    client: Client,
    endpoint: Url,
    maximum_amount: Option<u64>,
}

impl Service {
    pub fn new(
        endpoint: Url,
        chain_id: ChainId,
        faucet_account: LocalAccount,
        maximum_amount: Option<u64>,
    ) -> Self {
        let client = Client::new(endpoint.clone());
        Service {
            faucet_account: Mutex::new(faucet_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_gas_unit_price(std::cmp::max(1, aptos_global_constants::GAS_UNIT_PRICE))
                .with_transaction_expiration_time(30),
            client,
            endpoint,
            maximum_amount,
        }
    }

    // By default the path is prefixed with the version, e.g. `v1/`. The fake
    // API used in the faucet tests doesn't have a versioned API however, so
    // we just set it to `/`.
    pub fn configure_for_testing(mut self) -> Self {
        self.client = self.client.version_path_base("/".to_string()).unwrap();
        self
    }

    pub fn endpoint(&self) -> &Url {
        &self.endpoint
    }
}

pub fn routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let mint = mint::mint_routes(service.clone());
    let health = health_route(service);

    health
        .or(mint)
        .with(warp::log::custom(|info| {
            let forwarded_for = info
                .request_headers()
                .get("x-forwarded-for")
                .map(|inner| inner.to_str().unwrap_or("-"));

            info!(
                remote_addr = info.remote_addr(),
                forwarded_for = forwarded_for,
                host = info.host(),
                method = format!("{}", info.method()),
                path = info.path(),
                version = format!("{:?}", info.version()),
                status = format!("{}", info.status()),
                referer = info.referer(),
                user_agent = info.user_agent(),
                elapsed = info.elapsed(),
                "mint request"
            )
        }))
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_headers(vec![http::header::CONTENT_TYPE])
                .allow_methods(vec!["POST"]),
        )
}

fn health_route(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("health")
        .and(warp::get())
        .and(warp::any().map(move || service.clone()))
        .and_then(handle_health)
}

async fn handle_health(service: Arc<Service>) -> Result<Box<dyn warp::Reply>, Infallible> {
    let faucet_address = service.faucet_account.lock().await.address();
    let faucet_account = service.client.get_account(faucet_address).await;

    match faucet_account {
        Ok(account) => Ok(Box::new(account.inner().sequence_number.to_string())),
        Err(err) => Ok(Box::new(warp::reply::with_status(
            err.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
    }
}

/// The idea is that this may be happening concurrently. If we end up in such a race, the faucets
/// might attempt to send transactions with the same sequence number, in such an event, one will
/// succeed and the other will hit an unwrap. Eventually all faucets should get online.
pub async fn delegate_mint_account(
    service: Arc<Service>,
    server_url: Url,
    chain_id: ChainId,
    maximum_amount: Option<u64>,
) -> Arc<Service> {
    // Create a new random account, then delegate to it
    let mut delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);

    // Create the account
    let response = mint::process(
        &service,
        mint::MintParams {
            amount: 100_000_000_000,
            auth_key: None,
            address: Some(
                delegated_account
                    .authentication_key()
                    .clone()
                    .derived_address()
                    .to_hex_literal(),
            ),
            pub_key: None,
            return_txns: Some(true),
        },
    )
    .await
    .expect("Failed to create new account");

    match response {
        mint::Response::SubmittedTxns(txns) => {
            for txn in txns {
                service
                    .client
                    .wait_for_signed_transaction(&txn)
                    .await
                    .unwrap();
            }
        }
        _ => panic!("Expected a set of Response::SubmittedTxns"),
    }

    // Delegate minting to the account
    {
        let mut faucet_account = service.faucet_account.lock().await;
        service
            .client
            .submit_and_wait(&faucet_account.sign_with_transaction_builder(
                service.transaction_factory.payload(
                    aptos_stdlib::aptos_coin_delegate_mint_capability(delegated_account.address()),
                ),
            ))
            .await
            .expect("Failed to delegate minting to the new account");
    }

    // claim the capability!
    service
        .client
        .submit_and_wait(
            &delegated_account.sign_with_transaction_builder(
                service
                    .transaction_factory
                    .payload(aptos_stdlib::aptos_coin_claim_mint_capability()),
            ),
        )
        .await
        .unwrap();

    Arc::new(Service::new(
        server_url,
        chain_id,
        delegated_account,
        maximum_amount,
    ))
}
