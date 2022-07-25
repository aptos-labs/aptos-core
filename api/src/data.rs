// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::page::Page;
use crate::param::TransactionIdParam;
use crate::version::Version;
use crate::{context::Context, metrics::metrics, param::AddressParam};
use anyhow::Result;
use aptos_api_types::{Error, LedgerInfo, Response, TransactionId};
use std::collections::BTreeMap;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// GET /data/
pub fn data_get_ledger_info(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data")
        .and(warp::get())
        .and(context.filter())
        .and_then(get_latest_ledger_info)
        .with(metrics("data_ledger_info"))
        .boxed()
}

// GET /data/accounts/<account_id>
pub fn data_get_account(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam)
        .and(warp::get())
        .and(warp::query::<Version>())
        .and(context.filter())
        .and_then(get_account)
        .with(metrics("data_get_account"))
        .boxed()
}

// GET /data/accounts/<account_id>/resources
pub fn data_get_account_resources(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam / "resources")
        .and(warp::get())
        .and(warp::query::<Version>())
        .and(context.filter())
        .and_then(get_account_resources)
        .with(metrics("data_get_account_resources"))
        .boxed()
}

// GET /data/transactions/<version>
pub fn data_get_transaction(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "transactions" / TransactionIdParam)
        .and(warp::get())
        .and(context.filter())
        .and_then(get_transaction)
        .with(metrics("data_get_transaction"))
        .boxed()
}

// GET /data/transactions/
pub fn data_get_transactions(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "transactions")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(get_transactions)
        .with(metrics("data_get_transactions"))
        .boxed()
}

// GET /data/account/<account_id>/transactions/
pub fn data_get_account_transactions(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam / "transactions")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(get_account_transactions)
        .with(metrics("data_get_account_transactions"))
        .boxed()
}

// GET /data/account/<account_id>/transactions/
pub fn data_get_transaction_outputs(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "transaction_outputs")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(get_transaction_outputs)
        .with(metrics("data_get_transaction_outputs"))
        .boxed()
}

async fn get_latest_ledger_info(context: Context) -> Result<impl Reply, Rejection> {
    let raw_ledger_info = context.raw_ledger_info()?;
    let ledger_info = LedgerInfo::new(
        &context.chain_id(),
        &raw_ledger_info,
        context
            .db
            .get_first_txn_version()
            .map_err(Error::internal)?
            .unwrap_or(0),
    );

    Ok(Response::new(ledger_info, &raw_ledger_info)?)
}

async fn get_account(
    address_param: AddressParam,
    ledger_version: Version,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let account = address_param.parse("address")?;
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;
    let version = if let Some(ledger_version) = ledger_version.version {
        ledger_version.parse("ledger_version")?
    } else {
        latest_ledger_version
    };
    Ok(Response::new(
        ledger_info,
        &context.raw_account_state(*account.inner(), version)?,
    )?)
}

async fn get_account_resources(
    address_param: AddressParam,
    ledger_version: Version,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let account = address_param.parse("address")?;
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;
    let version = if let Some(ledger_version) = ledger_version.version {
        ledger_version.parse("ledger_version")?
    } else {
        latest_ledger_version
    };
    let account_state = context.raw_account_state(*account.inner(), version)?;
    let map: BTreeMap<_, _> = account_state.get_resources().collect();
    Ok(Response::new(ledger_info, &map)?)
}

async fn get_transaction(
    transaction_param: TransactionIdParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let transaction_id = transaction_param.parse("transaction hash or version")?;
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;

    Ok(match transaction_id {
        TransactionId::Hash(hash) => Response::new(
            ledger_info,
            &context.raw_transaction_by_hash(hash.into(), latest_ledger_version)?,
        )?,
        TransactionId::Version(version) => Response::new(
            ledger_info,
            &context.raw_transaction_by_version(version, latest_ledger_version)?,
        )?,
    })
}

async fn get_transactions(page: Page, context: Context) -> Result<impl Reply, Rejection> {
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;
    let (start, limit) = parse_page(page, latest_ledger_version)?;

    Ok(Response::new(
        ledger_info,
        &context.raw_transactions(start, limit, latest_ledger_version)?,
    )?)
}

async fn get_transaction_outputs(page: Page, context: Context) -> Result<impl Reply, Rejection> {
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;
    let (start, limit) = parse_page(page, latest_ledger_version)?;
    Ok(Response::new(
        ledger_info,
        &context.raw_transaction_outputs(start, limit, latest_ledger_version)?,
    )?)
}

async fn get_account_transactions(
    account: AddressParam,
    page: Page,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let account = account.parse("account")?.into();
    let ledger_info = context.get_latest_ledger_info()?;
    let latest_ledger_version = ledger_info.ledger_version.0;
    let start = page.start(0, u64::MAX)?;
    let limit = page.limit()?;
    Ok(Response::new(
        ledger_info,
        &context.raw_account_transactions(account, start, limit as u64, latest_ledger_version),
    )?)
}

fn parse_page(page: Page, latest_ledger_version: u64) -> Result<(u64, u64), Error> {
    let limit = page.limit()?;
    let last_page_start = if latest_ledger_version > (limit as u64) {
        latest_ledger_version - (limit as u64)
    } else {
        0
    };
    let start_version = page.start(last_page_start, latest_ledger_version)?;
    Ok((start_version, limit as u64))
}
