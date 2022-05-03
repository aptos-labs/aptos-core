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
use reqwest::StatusCode;
use std::{
    convert::Infallible,
    fmt,
    sync::{Arc, Mutex},
};
use url::Url;
use warp::{http, Filter, Rejection, Reply};

pub mod mint;

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
    let faucet_address = service.faucet_account.lock().unwrap().address();
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
        let mut faucet_account = service.faucet_account.lock().unwrap();
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
