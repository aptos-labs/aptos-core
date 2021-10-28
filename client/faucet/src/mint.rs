// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::Service;
use anyhow::Result;
use diem_logger::prelude::warn;
use diem_sdk::{
    client::MethodRequest,
    transaction_builder::Currency,
    types::{
        account_address::AccountAddress,
        account_config::{testnet_dd_account_address, treasury_compliance_account_address},
        transaction::{authenticator::AuthenticationKey, metadata, SignedTransaction},
    },
};
use reqwest::StatusCode;
use serde::Deserialize;
use std::{convert::Infallible, fmt, sync::Arc};
use warp::{Filter, Rejection, Reply};

pub fn mint_routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // POST /?amount=25&auth_key=xxx&currency_code=XXX
    // POST /mint?amount=25&auth_key=xxx&currency_code=XXX
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
    DDAccountNextSeqNum(u64),
    SubmittedTxns(Vec<SignedTransaction>),
}

impl std::fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::DDAccountNextSeqNum(v1) => write!(f, "{}", v1),
            Response::SubmittedTxns(v2) => {
                write!(f, "{}", hex::encode(bcs::to_bytes(&v2).unwrap()))
            }
        }
    }
}

#[derive(Deserialize)]
pub struct MintParams {
    pub amount: u64,
    pub currency_code: Currency,
    pub auth_key: AuthenticationKey,
    pub return_txns: Option<bool>,
    pub is_designated_dealer: Option<bool>,
    pub trade_id: Option<String>,
    pub vasp_domain: Option<String>,
    pub is_remove_domain: Option<bool>,
}

impl std::fmt::Display for MintParams {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.vasp_domain)
    }
}

impl MintParams {
    fn bcs_metadata(&mut self) -> Vec<u8> {
        match self.trade_id.take() {
            Some(trade_id) => {
                let metadata = metadata::Metadata::CoinTradeMetadata(
                    metadata::CoinTradeMetadata::CoinTradeMetadataV0(
                        metadata::CoinTradeMetadataV0 {
                            trade_ids: vec![trade_id],
                        },
                    ),
                );
                bcs::to_bytes(&metadata).unwrap_or_else(|e| {
                    warn!("Unable to serialize trade_id: {}", e);
                    vec![]
                })
            }
            _ => vec![],
        }
    }

    fn receiver(&self) -> AccountAddress {
        self.auth_key.derived_address()
    }
}

async fn process(service: &Service, mut params: MintParams) -> Result<Response> {
    let (tc_seq, dd_seq, receiver_seq) = sequences(service, params.receiver()).await?;
    let txns = {
        let mut treasury_account = service.treasury_compliance_account.lock().unwrap();
        let mut dd_account = service.designated_dealer_account.lock().unwrap();

        // If the onchain sequence_number is greater than what we have, update our
        // sequence_numbers
        if tc_seq > treasury_account.sequence_number() {
            *treasury_account.sequence_number_mut() = tc_seq;
        }
        if dd_seq > dd_account.sequence_number() {
            *dd_account.sequence_number_mut() = tc_seq;
        }

        let mut txns = vec![];
        if receiver_seq.is_none() {
            let builder = if params.is_designated_dealer.unwrap_or(false) {
                service.transaction_factory.create_designated_dealer(
                    params.currency_code,
                    0, // sliding_nonce
                    params.auth_key,
                    &format!("No. {} DD", treasury_account.sequence_number()),
                    false, // add all currencies
                )
            } else {
                service.transaction_factory.create_parent_vasp_account(
                    params.currency_code,
                    0, // sliding_nonce
                    params.auth_key,
                    &format!("No. {} VASP", treasury_account.sequence_number()),
                    false, // add all currencies
                )
            };

            txns.push(treasury_account.sign_with_transaction_builder(builder));
        }

        if let (Some(ref vasp_domain), Some(is_remove_domain)) =
            (&params.vasp_domain, params.is_remove_domain)
        {
            let builder = if is_remove_domain {
                service
                    .transaction_factory
                    .remove_vasp_domain(params.receiver(), vasp_domain.as_str().as_bytes().to_vec())
            } else {
                service
                    .transaction_factory
                    .add_vasp_domain(params.receiver(), vasp_domain.as_str().as_bytes().to_vec())
            };
            txns.push(treasury_account.sign_with_transaction_builder(builder));
        }

        txns.push(dd_account.sign_with_transaction_builder(
            service.transaction_factory.peer_to_peer_with_metadata(
                params.currency_code,
                params.receiver(),
                params.amount,
                params.bcs_metadata(),
                vec![],
            ),
        ));
        txns
    };

    let batch = txns
        .iter()
        .map(|txn| MethodRequest::submit(txn))
        .collect::<Result<_, _>>()
        .expect("serialization should not fail");
    let response = service.client.batch(batch).await?;

    // If there was an issue submitting a transaction we should just reset our sequence_numbers
    // to what was on chain
    if response.iter().any(Result::is_err) {
        *service
            .treasury_compliance_account
            .lock()
            .unwrap()
            .sequence_number_mut() = tc_seq;
        *service
            .designated_dealer_account
            .lock()
            .unwrap()
            .sequence_number_mut() = dd_seq;
    }

    if params.return_txns.unwrap_or(false) {
        Ok(Response::SubmittedTxns(txns))
    } else {
        Ok(Response::DDAccountNextSeqNum(
            service
                .designated_dealer_account
                .lock()
                .unwrap()
                .sequence_number(),
        ))
    }
}

async fn sequences(service: &Service, receiver: AccountAddress) -> Result<(u64, u64, Option<u64>)> {
    let accounts = vec![
        treasury_compliance_account_address(),
        testnet_dd_account_address(),
        receiver,
    ];
    let responses = service
        .client
        .batch(
            accounts
                .into_iter()
                .map(MethodRequest::get_account)
                .collect(),
        )
        .await?
        .into_iter()
        .map(|r| r.map_err(anyhow::Error::new))
        .map(|r| r.map(|response| response.into_inner().unwrap_get_account()))
        .collect::<Result<Vec<_>>>()?;
    let treasury_compliance = responses
        .get(0)
        .as_ref()
        .ok_or_else(|| anyhow::format_err!("get treasury compliance account response not found"))?
        .as_ref()
        .ok_or_else(|| anyhow::format_err!("treasury compliance account not found"))?
        .sequence_number;
    let designated_dealer = responses
        .get(1)
        .as_ref()
        .ok_or_else(|| anyhow::format_err!("get designated dealer account response not found"))?
        .as_ref()
        .ok_or_else(|| anyhow::format_err!("designated dealer account not found"))?
        .sequence_number;
    let receiver = responses
        .get(2)
        .as_ref()
        .ok_or_else(|| anyhow::format_err!("get receiver account response not found"))?
        .as_ref();
    let receiver_seq_num = receiver.map(|account| account.sequence_number);
    Ok((treasury_compliance, designated_dealer, receiver_seq_num))
}
