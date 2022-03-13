// Copyright (c) The Diem Core Contributors
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
use std::{convert::Infallible, fmt, ops::DerefMut, sync::Arc};
use tokio::sync::Mutex;

use url::Url;
use warp::{Filter, Rejection, Reply};

pub mod mint;

pub struct Service {
    pub faucet_account: Arc<Mutex<LocalAccount>>,
    pub transaction_factory: TransactionFactory,
    pub client: Client,
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
            faucet_account: Arc::new(Mutex::new(faucet_account)),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_gas_unit_price(1)
                .with_transaction_expiration_time(10),
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
    let accounts = accounts_routes(service.clone());
    let health = health_route(service);

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

fn accounts_routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    create_account_route(service.clone()).or(fund_account_route(service))
}

pub async fn delegate_account(
    service: Arc<Service>,
    server_url: String,
    chain_id: ChainId,
    maximum_amount: Option<u64>,
) -> Arc<Service> {
    // Create a new random account, then delegate to it
    let delegated_account = LocalAccount::generate(&mut rand::rngs::OsRng);

    // Create the account
    service
        .client
        .wait_for_signed_transaction(
            &create_account(
                service.clone(),
                CreateAccountParams {
                    pub_key: delegated_account.public_key().clone(),
                },
            )
            .await
            .unwrap(),
        )
        .await
        .unwrap();

    // Give the new account some moolah
    service
        .client
        .wait_for_signed_transaction(
            // we hold the world ransom for  one... hundred... billion... dollars
            &fund_account_unchecked(
                service.clone(),
                delegated_account.address(),
                100_000_000_000,
            )
            .await
            .unwrap(),
        )
        .await
        .unwrap();

    let service = {
        let mut faucet_account = service.faucet_account.lock().await;
        get_and_update_seq_no(&service, faucet_account.deref_mut())
            .await
            .unwrap();
        // Delegate minting to the account
        service
            .client
            .submit_and_wait(&faucet_account.sign_with_transaction_builder(
                service.transaction_factory.payload(
                    aptos_stdlib::encode_delegate_mint_capability_script_function(
                        delegated_account.address(),
                    ),
                ),
            ))
            .await
            .unwrap();

        Arc::new(Service::new(
            server_url,
            chain_id,
            delegated_account,
            maximum_amount,
        ))
    };
    {
        let mut faucet_account = service.faucet_account.lock().await;
        get_and_update_seq_no(&service, faucet_account.deref_mut())
            .await
            .unwrap();

        // claim the capability!
        service
            .client
            .submit_and_wait(
                &faucet_account.sign_with_transaction_builder(
                    service
                        .transaction_factory
                        .payload(aptos_stdlib::encode_claim_mint_capability_script_function()),
                ),
            )
            .await
            .unwrap();
    }
    service
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
    let mut faucet_account = service.faucet_account.lock().await;
    get_and_update_seq_no(&service, faucet_account.deref_mut()).await?;

    let txn = faucet_account.sign_with_transaction_builder(service.transaction_factory.payload(
        aptos_stdlib::encode_create_account_script_function(
            params.receiver(),
            params.pre_image().into_vec(),
        ),
    ));
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
    let asked_amount = params
        .amount
        .ok_or_else(|| anyhow::format_err!("Mint amount must be provided"))?;
    let service_amount = service.maximum_amount.unwrap_or(asked_amount);
    let amount = std::cmp::min(asked_amount, service_amount);
    fund_account_unchecked(service, address, amount).await
}

pub(crate) async fn fund_account_unchecked(
    service: Arc<Service>,
    address: AccountAddress,
    amount: u64,
) -> Result<SignedTransaction> {
    // Check to ensure the account has already been created
    if service.client.get_account(address).await.is_err() {
        return Err(anyhow!("account doesn't exist"));
    }

    let mut faucet_account = service.faucet_account.lock().await;
    get_and_update_seq_no(&service, faucet_account.deref_mut()).await?;

    let txn = faucet_account.sign_with_transaction_builder(
        service
            .transaction_factory
            .payload(aptos_stdlib::encode_mint_script_function(address, amount)),
    );

    service.client.submit(&txn).await?;
    Ok(txn)
}

pub(crate) async fn get_and_update_seq_no(
    service: &Service,
    faucet_account: &mut LocalAccount,
) -> Result<u64> {
    let faucet_account_address = faucet_account.address();
    let faucet_sequence_number = service
        .client
        .get_account(faucet_account_address)
        .await
        .map_err(|_| anyhow::format_err!("faucet account {} not found", faucet_account_address))?
        .into_inner()
        .sequence_number;

    // If the onchain sequence_number is greater than what we have, update our sequence_numbers
    if faucet_sequence_number > faucet_account.sequence_number() {
        *faucet_account.sequence_number_mut() = faucet_sequence_number;
    }
    Ok(faucet_sequence_number)
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
