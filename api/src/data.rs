// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! These APIs get raw BCS from the DB.  It's to allow conversion from raw types directly
//! from Move types on the client side.
//!
//! TODO: Add support for querying for specific resources or events
//! TODO: Add support for querying tables
//! TODO: Add support in OpenAPI spec
//! TODO: Move to Poem
//! TODO: Add support for blocks

use crate::accounts::Account;
use crate::page::Page;
use crate::param::LedgerVersionParam;
use crate::transactions::Transactions;
use crate::version::Version;
use crate::{context::Context, metrics::metrics, param::AddressParam};
use anyhow::Result;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// GET /data/accounts/<account_id>
pub fn data_get_account(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam)
        .and(warp::get())
        .and(context.filter())
        .and_then(get_account)
        .with(metrics("data_get_account"))
        .boxed()
}

// GET /data/accounts/<account_id>/resources
pub fn data_get_account_resources(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam / "resources")
        .and(warp::get())
        .and(context.filter())
        .and(warp::query::<Version>())
        .map(|address, ctx, version: Version| (version.version, address, ctx))
        .untuple_one()
        .and_then(get_account_resources)
        .with(metrics("data_get_account_resources"))
        .boxed()
}

// GET /data/accounts/<account_id>/modules
pub fn data_get_account_modules(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam / "modules")
        .and(warp::get())
        .and(context.filter())
        .and(warp::query::<Version>())
        .map(|address, ctx, version: Version| (version.version, address, ctx))
        .untuple_one()
        .and_then(get_account_modules)
        .with(metrics("data_get_account_modules"))
        .boxed()
}

// GET /data/accounts/<account_id>/transactions?start={u64}&limit={u16}
pub fn data_get_account_transactions(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "accounts" / AddressParam / "transactions")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(get_account_transactions)
        .with(metrics("data_get_account_transactions"))
        .boxed()
}

// GET /transactions?start={u64}&limit={u16}
pub fn data_get_transactions(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("data" / "transactions")
        .and(warp::get())
        .and(warp::query::<Page>())
        .and(context.filter())
        .and_then(get_transactions)
        .with(metrics("data_get_transactions"))
        .boxed()
}

async fn get_account(address: AddressParam, context: Context) -> Result<impl Reply, Rejection> {
    Ok(Account::new(None, address, context)?.raw_account_resource()?)
}

async fn get_account_resources(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(ledger_version, address, context)?.raw_resources()?)
}

async fn get_account_modules(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(ledger_version, address, context)?.raw_modules()?)
}

async fn get_account_transactions(
    address: AddressParam,
    page: Page,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Transactions::new(context)?.raw_list_by_account(address, page)?)
}

async fn get_transactions(page: Page, context: Context) -> Result<impl Reply, Rejection> {
    Ok(Transactions::new(context)?.raw_list(page)?)
}
