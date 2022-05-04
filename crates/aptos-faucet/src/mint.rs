// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::Service;
use anyhow::Result;
use aptos_crypto::{ed25519::Ed25519PublicKey, hash::HashValue};
use aptos_logger::{error, info, warn};
use aptos_sdk::{
    transaction_builder::aptos_stdlib,
    types::{
        account_address::AccountAddress,
        transaction::{authenticator::AuthenticationKey, SignedTransaction},
    },
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::{convert::Infallible, fmt, sync::Arc};
use warp::{Filter, Rejection, Reply};

pub fn mint_routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // POST /?amount=25&address=xxx
    // POST /mint?amount=25&address=xxx
    warp::path::end()
        .or(warp::path::path("mint"))
        .and(warp::post())
        .and(warp::any().map(move || service.clone()))
        .and(warp::query().map(move |params: MintParams| params))
        .and_then(|_, service, params| handle(service, params))
}

async fn handle(
    service: Arc<Service>,
    params: MintParams,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    match process(&service, params).await {
        Ok(body) => Ok(Box::new(body.to_string())),
        Err(err) => Ok(Box::new(warp::reply::with_status(
            err.to_string(),
            StatusCode::INTERNAL_SERVER_ERROR,
        ))),
    }
}

#[derive(Debug)]
pub enum Response {
    SubmittedTxns(Vec<SignedTransaction>),
    SubmittedTxnsHashes(Vec<HashValue>),
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::SubmittedTxns(value) => {
                write!(f, "{}", hex::encode(bcs::to_bytes(&value).unwrap()))
            }
            Response::SubmittedTxnsHashes(value) => {
                write!(f, "{}", serde_json::to_string(&value).unwrap())
            }
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct MintParams {
    pub amount: u64,
    pub auth_key: Option<String>,
    pub address: Option<String>,
    pub pub_key: Option<Ed25519PublicKey>,
    pub return_txns: Option<bool>,
}

impl std::fmt::Display for MintParams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<Mint {:?} to {:?}>", self.amount, self.receiver())
    }
}

impl MintParams {
    fn receiver(&self) -> Option<AccountAddress> {
        if let Some(auth_key) = self.auth_key.as_ref() {
            return match AccountAddress::from_hex_literal(auth_key) {
                Ok(auth_key) => Some(auth_key),
                Err(_) => match AccountAddress::from_hex(auth_key) {
                    Ok(auth_key) => Some(auth_key),
                    Err(_) => None,
                },
            };
        }
        if let Some(address) = self.address.as_ref() {
            return match AccountAddress::from_hex_literal(address) {
                Ok(address) => Some(address),
                Err(_) => match AccountAddress::from_hex(address) {
                    Ok(address) => Some(address),
                    Err(_) => None,
                },
            };
        }
        if let Some(pub_key) = self.pub_key.as_ref() {
            return Some(AuthenticationKey::ed25519(pub_key).derived_address());
        }
        None
    }
}

pub async fn process(service: &Service, params: MintParams) -> Result<Response> {
    let maybe_maximum_amount = service.maximum_amount.unwrap_or(params.amount);
    let amount = std::cmp::min(params.amount, maybe_maximum_amount);

    let receiver_address = params.receiver().ok_or_else(|| {
        anyhow::format_err!("You must provide 'address' (preferred), 'pub_key', or 'auth_key'")
    })?;

    let (mut faucet_seq, mut receiver_seq) = sequences(service, receiver_address).await?;
    let our_faucet_seq = {
        let mut faucet_account = service.faucet_account.lock().unwrap();

        // If the onchain sequence_number is greater than what we have, update our
        // sequence_numbers
        if faucet_seq > faucet_account.sequence_number() {
            *faucet_account.sequence_number_mut() = faucet_seq;
        }
        faucet_account.sequence_number()
    };

    // We shouldn't have too many outstanding txns
    for _ in 0..60 {
        if our_faucet_seq < faucet_seq + 50 {
            break;
        }
        warn!(
            "We have too many outstanding transactions: {}. Sleeping to let the system catchup.",
            (our_faucet_seq - faucet_seq)
        );

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        let (lhs, rhs) = sequences(service, receiver_address).await?;
        faucet_seq = lhs;
        receiver_seq = rhs;
    }

    // After 30 seconds, we still have not caught up, we are likely unhealthy
    if our_faucet_seq >= faucet_seq + 50 {
        error!("We are unhealthy, transactions have likely expired.");
        let mut faucet_account = service.faucet_account.lock().unwrap();
        if faucet_account.sequence_number() >= faucet_seq + 50 {
            info!("Resetting the sequence number counter.");
            *faucet_account.sequence_number_mut() = faucet_seq;
        } else {
            info!("Someone else reset the sequence number counter ahead of us.");
        }
    }

    let mut txns = vec![];

    {
        let mut faucet_account = service.faucet_account.lock().unwrap();

        if receiver_seq.is_none() {
            let builder =
                service
                    .transaction_factory
                    .payload(aptos_stdlib::encode_account_create_account(
                        receiver_address,
                    ));

            let txn = faucet_account.sign_with_transaction_builder(builder);
            txns.push(txn)
        }

        if amount != 0 {
            txns.push(
                faucet_account.sign_with_transaction_builder(service.transaction_factory.payload(
                    aptos_stdlib::encode_test_coin_mint(receiver_address, amount),
                )),
            );
        }
    }

    let requests = txns.iter().map(|txn| service.client.submit(txn));
    let mut responses = futures::future::join_all(requests).await;

    // If there was an issue submitting a transaction we should just reset our sequence_numbers
    // to what was on chain
    if responses.iter().any(Result::is_err) {
        *service.faucet_account.lock().unwrap().sequence_number_mut() = faucet_seq;
    }

    while !responses.is_empty() {
        let response = responses.swap_remove(0);
        response?;
    }

    if params.return_txns.unwrap_or(false) {
        Ok(Response::SubmittedTxns(txns))
    } else {
        let hashes = txns
            .iter()
            .map(|txn| txn.clone().committed_hash())
            .collect();
        Ok(Response::SubmittedTxnsHashes(hashes))
    }
}

async fn sequences(service: &Service, receiver: AccountAddress) -> Result<(u64, Option<u64>)> {
    let faucet_address = service.faucet_account.lock().unwrap().address();
    let f_request = service.client.get_account(faucet_address);
    let r_request = service.client.get_account(receiver);
    let mut responses = futures::future::join_all([f_request, r_request]).await;

    let receiver_seq_num = responses
        .remove(1)
        .as_ref()
        .ok()
        .map(|account| account.inner().sequence_number);
    let faucet_seq_num = responses
        .remove(0)
        .map_err(|_| anyhow::format_err!("faucet account {} not found", faucet_address))?
        .inner()
        .sequence_number;

    Ok((faucet_seq_num, receiver_seq_num))
}
