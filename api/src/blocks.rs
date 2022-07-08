// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, failpoint::fail_point, metrics::metrics, param::LedgerVersionParam};
use anyhow::Result;
use aptos_api_types::{Error, LedgerInfo, Response, TransactionId};
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
                TransactionId::Version(ledger_version),
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
