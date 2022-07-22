// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::accept_type::{parse_accept, AcceptType};
use super::page::Page;
use super::AptosErrorCode;
use super::{
    ApiTags, AptosErrorResponse, BasicErrorWith404, BasicResponse, BasicResponseStatus,
    BasicResultWith404, InternalError,
};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{AsConverter, LedgerInfo, Transaction, TransactionOnChainData};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::OpenApi;

pub struct TransactionsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl TransactionsApi {
    /// Get transactions
    ///
    /// todo
    #[oai(
        path = "/transactions",
        method = "get",
        operation_id = "get_transactions",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transactions(
        &self,
        accept: Accept,
        start: Query<Option<u64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Transaction>> {
        fail_point_poem("endppoint_get_transactions")?;
        let accept_type = parse_accept(&accept)?;
        let page = Page::new(start.0, limit.0);
        self.list(&accept_type, page)
    }
}

impl TransactionsApi {
    fn list(&self, accept_type: &AcceptType, page: Page) -> BasicResultWith404<Vec<Transaction>> {
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        let ledger_version = latest_ledger_info.version();
        let limit = page.limit()?;
        let last_page_start = if ledger_version > (limit as u64) {
            ledger_version - (limit as u64)
        } else {
            0
        };
        let start_version = page.start(last_page_start, ledger_version)?;

        let data = self
            .context
            .get_transactions(start_version, limit, ledger_version)
            .context("Failed to read raw transactions from storage")
            .map_err(BasicErrorWith404::internal)
            .map_err(|e| e.error_code(AptosErrorCode::InvalidBcsInStorageError))?;

        self.render_transactions(data, accept_type, &latest_ledger_info)
    }

    fn render_transactions(
        &self,
        data: Vec<TransactionOnChainData>,
        accept_type: &AcceptType,
        latest_ledger_info: &LedgerInfo,
    ) -> BasicResultWith404<Vec<Transaction>> {
        if data.is_empty() {
            let data: Vec<Transaction> = vec![];
            return BasicResponse::try_from_rust_value((
                data,
                latest_ledger_info,
                BasicResponseStatus::Ok,
                accept_type,
            ));
        }

        let resolver = self.context.move_resolver_poem()?;
        let converter = resolver.as_converter();
        let txns: Vec<Transaction> = data
            .into_iter()
            .map(|t| {
                let version = t.version;
                let timestamp = self.context.get_block_timestamp(version)?;
                let txn = converter.try_into_onchain_transaction(timestamp, t)?;
                Ok(txn)
            })
            .collect::<Result<_, anyhow::Error>>()
            .context("Failed to convert transaction data from storage")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            txns,
            latest_ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }
}
