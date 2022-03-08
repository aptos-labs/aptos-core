// Copyright (c) The Aptos Foundation
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

use anyhow::{anyhow, Result};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    transaction_builder::{aptos_stdlib, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
            SignedTransaction,
        },
        LocalAccount,
    },
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::{
    convert::Infallible,
    fmt,
    sync::{Arc, Mutex},
};
use url::Url;
use warp::{Filter, Rejection, Reply};

pub mod mint;

pub struct Service {
    pub faucet_account: Mutex<LocalAccount>,
    transaction_factory: TransactionFactory,
    client: Client,
    endpoint: String,
    fixed_amount: Option<u64>,
}

impl Service {
    pub fn new(
        endpoint: String,
        chain_id: ChainId,
        faucet_account: LocalAccount,
        fixed_amount: Option<u64>,
    ) -> Self {
        let client = Client::new(Url::parse(&endpoint).expect("Invalid rest endpoint"));
        Service {
            faucet_account: Mutex::new(faucet_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_transaction_expiration_time(30),
            client,
            endpoint,
            fixed_amount,
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
    let accounts = accounts_routes(service);
    let health = warp::path!("-" / "healthy").map(|| "aptos-faucet:ok");

    health
        .or(mint)
        .or(accounts)
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
        .with(warp::cors().allow_any_origin().allow_methods(vec!["POST"]))
}

fn accounts_routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_account_route(service.clone()).or(fund_account_route(service))
}

#[derive(Deserialize)]
struct CreateAccountParams {
    pub_key: Ed25519PublicKey,
}

impl CreateAccountParams {
    fn pre_image(&self) -> AuthenticationKeyPreimage {
        AuthenticationKeyPreimage::ed25519(&self.pub_key)
    }

    fn receiver(&self) -> AccountAddress {
        AuthenticationKey::ed25519(&self.pub_key).derived_address()
    }
}
fn create_account_route(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path::path("accounts")
        .and(warp::post())
        .and(warp::any().map(move || service.clone()))
        .and(warp::query().map(move |params: CreateAccountParams| params))
        .and_then(handle_create_account)
}

async fn handle_create_account(
    service: Arc<Service>,
    params: CreateAccountParams,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match create_account(service, params).await {
        Ok(txn) => Ok(Box::new(hex::encode(bcs::to_bytes(&txn).unwrap()))),
        Err(err) => Ok(Box::new(warp::reply::with_status(
            err.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
    }
}

async fn create_account(
    service: Arc<Service>,
    params: CreateAccountParams,
) -> Result<SignedTransaction> {
    // Check to ensure the account hasn't already been created
    if service.client.get_account(params.receiver()).await.is_ok() {
        return Err(anyhow!("account already exists"));
    }

    let faucet_account_address = service.faucet_account.lock().unwrap().address();
    let faucet_sequence_number = service
        .client
        .get_account(faucet_account_address)
        .await
        .map_err(|_| anyhow::format_err!("faucet account {} not found", faucet_account_address))?
        .into_inner()
        .sequence_number;

    let txn = {
        let mut faucet_account = service.faucet_account.lock().unwrap();
        if faucet_sequence_number > faucet_account.sequence_number() {
            *faucet_account.sequence_number_mut() = faucet_sequence_number;
        }

        let builder = service.transaction_factory.payload(
            aptos_stdlib::encode_create_account_script_function(
                params.receiver(),
                params.pre_image().into_vec(),
            ),
        );

        faucet_account.sign_with_transaction_builder(builder)
    };

    service.client.submit(&txn).await?;
    Ok(txn)
}

#[derive(Deserialize)]
struct FundAccountParams {
    amount: Option<u64>,
}

fn fund_account_route(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / AccountAddress / "fund")
        .and(warp::post())
        .and(warp::any().map(move || service.clone()))
        .and(warp::query().map(move |params: FundAccountParams| params))
        .and_then(handle_fund_account)
}

async fn handle_fund_account(
    address: AccountAddress,
    service: Arc<Service>,
    params: FundAccountParams,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match fund_account(service, address, params).await {
        Ok(txn) => Ok(Box::new(hex::encode(bcs::to_bytes(&txn).unwrap()))),
        Err(err) => Ok(Box::new(warp::reply::with_status(
            err.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
    }
}

async fn fund_account(
    service: Arc<Service>,
    address: AccountAddress,
    params: FundAccountParams,
) -> Result<SignedTransaction> {
    if service.fixed_amount.is_some() && params.amount.is_some() {
        return Err(anyhow::format_err!(
            "Mint amount is fixed to {} on this faucet",
            service.fixed_amount.unwrap()
        ));
    }

    if service.fixed_amount.is_none() && params.amount.is_none() {
        return Err(anyhow::format_err!("Mint amount must be provided"));
    }

    let amount = service
        .fixed_amount
        .unwrap_or_else(|| params.amount.unwrap());

    // Check to ensure the account has already been created
    if service.client.get_account(address).await.is_err() {
        return Err(anyhow!("account doesn't exist"));
    }

    let faucet_account_address = service.faucet_account.lock().unwrap().address();
    let faucet_sequence_number = service
        .client
        .get_account(faucet_account_address)
        .await
        .map_err(|_| anyhow::format_err!("faucet account {} not found", faucet_account_address))?
        .into_inner()
        .sequence_number;

    let txn = {
        let mut faucet_account = service.faucet_account.lock().unwrap();
        if faucet_sequence_number > faucet_account.sequence_number() {
            *faucet_account.sequence_number_mut() = faucet_sequence_number;
        }

        faucet_account.sign_with_transaction_builder(
            service
                .transaction_factory
                .payload(aptos_stdlib::encode_mint_script_function(address, amount)),
        )
    };

    service.client.submit(&txn).await?;
    Ok(txn)
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
