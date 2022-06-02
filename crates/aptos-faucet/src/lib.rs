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
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{chain_id::ChainId, LocalAccount},
};
use futures::lock::Mutex;
use reqwest::StatusCode;
use std::{convert::Infallible, fmt, sync::Arc};
use std::path::Path;
use url::Url;
use warp::{http, Filter, Rejection, Reply};
use aptos::common::types::EncodingType;
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::types::account_address::AccountAddress;
use aptos_sdk::types::account_config::aptos_root_address;
use structopt::StructOpt;

pub mod mint;

#[derive(Debug, StructOpt)]
#[structopt(
name = "Aptos Faucet",
author = "Aptos",
about = "Aptos Testnet utility service for creating test accounts and minting test coins"
)]
pub struct FaucetArgs {
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
    /// Ed25519PrivateKey for minting coins
    #[structopt(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,
    /// Address of the account to send transactions from.
    /// On Testnet, for example, this is a550c18.
    /// If not present, the mint key's address is used
    #[structopt(short = "t", long, parse(try_from_str = AccountAddress::from_hex_literal))]
    pub mint_account_address: Option<AccountAddress>,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: "MAINNET" or 1, testnet: "TESTNET" or 2, devnet: "DEVNET" or 3,
    /// local swarm: "TESTING" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[structopt(short = "c", long, default_value = "2")]
    pub chain_id: ChainId,
    /// Maximum amount of coins to mint.
    #[structopt(long)]
    pub maximum_amount: Option<u64>,
    #[structopt(long)]
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

        let key = if let Some(key) = self.mint_key {
            key.private_key()
        } else {
            EncodingType::BCS
                .load_key::<Ed25519PrivateKey>("mint key", Path::new(&self.mint_key_file_path))
                .unwrap()
        };

        let faucet_address: AccountAddress =
            self.mint_account_address.unwrap_or_else(aptos_root_address);
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
            delegate_mint_account(
                service,
                self.server_url,
                self.chain_id,
                self.maximum_amount,
            )
                .await
        };

        info!(
        "[faucet]: running on: {}. Minting from {}",
        address,
        actual_service.faucet_account.lock().await.address()
    );
        warp::serve(routes(actual_service))
            .run(address)
            .await;
    }
}


pub struct Service {
    pub faucet_account: Mutex<LocalAccount>,
    transaction_factory: TransactionFactory,
    client: Client,
    endpoint: String,
    maximum_amount: Option<u64>,
}

impl Service {
    pub fn new(
        endpoint: String,
        chain_id: ChainId,
        faucet_account: LocalAccount,
        maximum_amount: Option<u64>,
    ) -> Self {
        let client = Client::new(Url::parse(&endpoint).expect("Invalid rest endpoint"));
        Service {
            faucet_account: Mutex::new(faucet_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_gas_unit_price(1)
                .with_transaction_expiration_time(30),
            client,
            endpoint,
            maximum_amount,
        }
    }

    pub fn endpoint(&self) -> &String {
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
            info!(
                "{} \"{} {} {:?}\" {} \"{}\" \"{}\" {:?}",
                OptFmt(info.remote_addr()),
                info.method(),
                info.path(),
                info.version(),
                info.status().as_u16(),
                OptFmt(info.referer()),
                OptFmt(info.user_agent()),
                info.elapsed(),
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

//
// Common Types
//

struct OptFmt<T>(Option<T>);

impl<T: fmt::Display> fmt::Display for OptFmt<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(t) = &self.0 {
            fmt::Display::fmt(t, f)
        } else {
            f.write_str("-")
        }
    }
}

/// The idea is that this may be happening concurrently. If we end up in such a race, the faucets
/// might attempt to send transactions with the same sequence number, in such an event, one will
/// succeed and the other will hit an unwrap. Eventually all faucets should get online.
pub async fn delegate_mint_account(
    service: Arc<Service>,
    server_url: String,
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
    .unwrap();

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
                    aptos_stdlib::encode_test_coin_delegate_mint_capability(
                        delegated_account.address(),
                    ),
                ),
            ))
            .await
            .unwrap();
    }

    // claim the capability!
    service
        .client
        .submit_and_wait(
            &delegated_account.sign_with_transaction_builder(
                service
                    .transaction_factory
                    .payload(aptos_stdlib::encode_test_coin_claim_mint_capability()),
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
