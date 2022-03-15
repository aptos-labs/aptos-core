// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{get_and_update_seq_no, Service, ServiceObject, ServicePool};
use anyhow::Result;
use aptos_crypto::{ed25519::Ed25519PublicKey, hash::HashValue};
use aptos_sdk::{
    transaction_builder::aptos_stdlib,
    types::{
        account_address::AccountAddress,
        transaction::{
            authenticator::{AuthenticationKey, AuthenticationKeyPreimage},
            SignedTransaction,
        },
    },
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::{convert::Infallible, fmt, ops::DerefMut, sync::Arc};
use warp::{Filter, Rejection, Reply};

pub fn mint_routes(
    service: ServicePool,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // POST /?amount=25&pub_key=xxx
    // POST /mint?amount=25&pub_key=xxx
    warp::path::end()
        .or(warp::path::path("mint"))
        .and(warp::post())
        .and(warp::any().map(move || service.clone()))
        .and(warp::query().map(move |params: MintParams| params))
        .and_then(|_, service, params| handle(service, params))
}

async fn handle(
    service: ServicePool,
    params: MintParams,
) -> Result<Box<dyn warp::Reply>, Infallible> {
    let service = service.clone().get().await.unwrap();
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
    pub amount: Option<u64>,
    pub pub_key: Ed25519PublicKey,
    pub return_txns: Option<bool>,
}

impl std::fmt::Display for MintParams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<Mint {:?} to {:?}>", self.amount, self.receiver())
    }
}

impl MintParams {
    fn pre_image(&self) -> AuthenticationKeyPreimage {
        AuthenticationKeyPreimage::ed25519(&self.pub_key)
    }

    fn receiver(&self) -> AccountAddress {
        AuthenticationKey::ed25519(&self.pub_key).derived_address()
    }
}

async fn process(service: &Service, params: MintParams) -> Result<Response> {
    let asked_amount = params
        .amount
        .ok_or_else(|| anyhow::format_err!("Mint amount must be provided"))?;
    let service_amount = service.maximum_amount.unwrap_or(asked_amount);
    let amount = std::cmp::min(asked_amount, service_amount);

    let mut faucet_account = service.faucet_account.lock().await;
    //let faucet_seq = get_and_update_seq_no(service, faucet_account.deref_mut()).await?;

    let mut txns = vec![];

    let receiver_seq = service
        .client
        .get_account(params.receiver())
        .await
        .ok()
        .map(|account| account.inner().sequence_number);

    if receiver_seq.is_none() {
        txns.push(faucet_account.sign_with_transaction_builder(
            service.transaction_factory.payload(
                aptos_stdlib::encode_create_account_script_function(
                    params.receiver(),
                    params.pre_image().into_vec(),
                ),
            ),
        ))
    }

    txns.push(
        faucet_account.sign_with_transaction_builder(service.transaction_factory.payload(
            aptos_stdlib::encode_mint_script_function(params.receiver(), amount),
        )),
    );

    let requests = txns.iter().map(|txn| service.client.submit(txn));
    let mut responses = futures::future::join_all(requests).await;

    // If there was an issue submitting a transaction we should just reset our sequence_numbers
    // to what was on chain
    if responses.iter().any(Result::is_err) {
        //*faucet_account.sequence_number_mut() = faucet_seq;
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
