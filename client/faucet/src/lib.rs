// Copyright (c) The Diem Core Contributors
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
//! cargo run -p diem-faucet -- -a 127.0.0.1 -p 8080 -c TESTNET \
//!     -m <treasury-compliance-private-key-path> -s https://testnet.diem.com/v1
//! ```
//!
//! Check help doc for options details:
//!
//! ```bash
//! cargo run -p diem-faucet -- -h
//! ```
//!
//! ## Development
//!
//! Test with diem-swarm by add -m option:
//!
//! ```bash
//! cargo run -p diem-swarm -- -s -l -n 1 -m
//! ```
//!

use anyhow::Result;
use diem_logger::info;
use diem_sdk::{
    client::{Client, SignedTransaction},
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, LocalAccount},
};
use std::{
    fmt,
    sync::{Arc, Mutex},
};
use warp::{Filter, Rejection, Reply};

pub mod mint;

pub struct Service {
    treasury_compliance_account: Mutex<LocalAccount>,
    designated_dealer_account: Mutex<LocalAccount>,
    transaction_factory: TransactionFactory,
    client: Client,
}

impl Service {
    pub fn new(
        jsonrpc_endpoint: String,
        chain_id: ChainId,
        treasury_compliance_account: LocalAccount,
        designated_dealer_account: LocalAccount,
    ) -> Self {
        let client = Client::new(jsonrpc_endpoint);
        Service {
            treasury_compliance_account: Mutex::new(treasury_compliance_account),
            designated_dealer_account: Mutex::new(designated_dealer_account),
            transaction_factory: TransactionFactory::new(chain_id)
                .with_transaction_expiration_time(30),
            client,
        }
    }
}

pub fn routes(
    service: Arc<Service>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let mint = mint::mint_routes(service.clone());
    let health = warp::path!("-" / "healthy").map(|| "diem-faucet:ok");

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
        .with(warp::cors().allow_any_origin().allow_methods(vec!["POST"]))
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
