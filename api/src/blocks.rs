// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// READ ME
// READ ME
// READ ME
// READ ME
//
// If you plan to change this file, please speak to @dport or @gregnazario
// first. This file contains endpoint handlers for the v0 API. This API is
// now locked except for critical security fixes. All new feature development
// should be made to the v1 API. You can find this code under `poem_backend`.
// The v0 API is deprecated and will be removed in September 1st.
// See https://github.com/aptos-labs/aptos-core/issues/2590
//
// READ ME
// READ ME
// READ ME
// READ ME

use crate::{context::Context, failpoint::fail_point, metrics::metrics, param::LedgerVersionParam};
use anyhow::Result;
use aptos_api_types::{Error, LedgerInfo, Response, TransactionId, U64};
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// GET /blocks/<version>
pub fn get_block_info(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("blocks" / LedgerVersionParam)
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_block_info)
        .with(metrics("get_block_info"))
        .boxed()
}

async fn handle_get_block_info(
    ledger_version: LedgerVersionParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_block_info")?;
    Ok(Block::new(Some(ledger_version), context)?.find_block()?)
}

pub(crate) struct Block {
    version: u64,
    latest_ledger_info: LedgerInfo,
    context: Context,
}

impl Block {
    pub fn new(
        ledger_version: Option<LedgerVersionParam>,
        context: Context,
    ) -> Result<Self, Error> {
        let latest_ledger_info = context.get_latest_ledger_info()?;
        let ledger_version = ledger_version
            .map(|v| v.parse("ledger version"))
            .unwrap_or_else(|| Ok(latest_ledger_info.version()))?;

        if ledger_version > latest_ledger_info.version() {
            return Err(Error::not_found(
                "ledger",
                TransactionId::Version(U64::from(ledger_version)),
                latest_ledger_info.version(),
            ));
        }

        Ok(Self {
            version: ledger_version,
            latest_ledger_info,
            context,
        })
    }

    /// Scans the DB for block boundaries, then retrieves all the transactions associated
    pub fn find_block(self) -> Result<Response, Error> {
        let ledger_version = self.latest_ledger_info.version();
        Response::new(
            self.latest_ledger_info,
            &self.context.get_block_info(self.version, ledger_version)?,
        )
    }
}
