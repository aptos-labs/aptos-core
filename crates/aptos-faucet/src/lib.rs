// Copyright (c) The Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crates provides Faucet service for creating Testnet with simplified on-chain account creation
//! and minting coins.
//!
//! THIS SERVICE SHOULD NEVER BE DEPLOYED TO MAINNET.
//!
//! ## Launch service
//!
//! Launch faucet service local and connect to Testnet:
//!
//! ```bash
//! cargo run -p aptos-faucet -- -a 127.0.0.1 -p 8080 -c TESTNET \
//!     -m <treasury-compliance-private-key-path> -s https://testnet.aptos-labs.com/v1
//! ```
//!
//! Check help doc for options details:
//!
//! ```bash
//! cargo run -p aptos-faucet -- -h
//! ```

use anyhow::{anyhow, Result};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    transaction_builder::{Currency, TransactionFactory},
    types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, SignedTransaction},
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
    treasury_compliance_account: Mutex<LocalAccount>,
    designated_dealer_account: Mutex<LocalAccount>,
    transaction_factory: TransactionFactory,
    client: Client,
    endpoint: String,
}

impl Service {
    pub fn new(
        endpoint: String,
        chain_id: ChainId,
        treasury_compliance_account: LocalAccount,
        designated_dealer_account: LocalAccount,
    ) -> Self {
        let client = Client::new(Url::parse(&endpoint).expect("Invalid rest endpont"));
        Service {
            treasury_compliance_account: Mutex::new(treasury_compliance_account),
            designated_dealer_account: Mutex::new(designated_dealer_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_transaction_expiration_time(30),
            client,
            endpoint,
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
#[serde(rename_all = "kebab-case")]
struct CreateAccountParams {
    authentication_key: AuthenticationKey,
    currency: Currency,
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
    if service
        .client
        .get_account(params.authentication_key.derived_address())
        .await
        .is_ok()
    {
        return Err(anyhow!("account already exists"));
    }

    // get TC account's sequence number
    let tc_account_address = service
        .treasury_compliance_account
        .lock()
        .unwrap()
        .address();
    let tc_sequence_number = service
        .client
        .get_account(tc_account_address)
        .await
        .map_err(|_| anyhow::format_err!("treasury compliance account not found"))?
        .into_inner()
        .sequence_number;

    let txn = {
        let mut treasury_account = service.treasury_compliance_account.lock().unwrap();
        if tc_sequence_number > treasury_account.sequence_number() {
            *treasury_account.sequence_number_mut() = tc_sequence_number;
        }

        let builder = service.transaction_factory.create_parent_vasp_account(
            params.currency,
            0, // sliding_nonce
            params.authentication_key,
            &format!("No. {}", treasury_account.sequence_number()),
            false, // add all currencies
        );

        treasury_account.sign_with_transaction_builder(builder)
    };

    service.client.submit(&txn).await?;
    Ok(txn)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct FundAccountParams {
    amount: u64,
    currency: Currency,
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
    // Check to ensure the account has already been created
    if service.client.get_account(address).await.is_err() {
        return Err(anyhow!("account doesn't exist"));
    }

    // get DD account's sequence number
    let dd_account_address = service.designated_dealer_account.lock().unwrap().address();
    let dd_sequence_number = service
        .client
        .get_account(dd_account_address)
        .await
        .map_err(|_| anyhow::format_err!("treasury compliance account not found"))?
        .into_inner()
        .sequence_number;

    let txn = {
        let mut dd_account = service.designated_dealer_account.lock().unwrap();
        if dd_sequence_number > dd_account.sequence_number() {
            *dd_account.sequence_number_mut() = dd_sequence_number;
        }

        dd_account.sign_with_transaction_builder(
            service.transaction_factory.peer_to_peer_with_metadata(
                params.currency,
                address,
                params.amount,
                vec![],
                vec![],
            ),
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
