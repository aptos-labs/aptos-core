// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;
use std::sync::Arc;

use super::accept_type::AcceptType;
use super::page::Page;
use super::{response::AptosResponseResult, ApiTags, AptosResponse};
use super::{AptosError, AptosErrorCode, AptosErrorResponse};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::format_err;
use aptos_api_types::AsConverter;
use aptos_api_types::{LedgerInfo, Transaction, TransactionOnChainData};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::OpenApi;

// TODO: Make a helper that builds an AptosResponse from just an anyhow error,
// that assumes that it's an internal error. We can use .context() add more info.

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
        tag = "ApiTags::General"
    )]
    async fn get_transactions(
        &self,
        accept: Accept,
        start: Query<Option<u64>>,
        limit: Query<Option<u16>>,
    ) -> AptosResponseResult<Vec<Transaction>> {
        fail_point_poem("endpoint_get_transactions")?;
        let accept_type = AcceptType::try_from(&accept)?;
        let page = Page::new(start.0, limit.0);
        self.list(&accept_type, page)
    }
}

impl TransactionsApi {
    fn list(&self, accept_type: &AcceptType, page: Page) -> AptosResponseResult<Vec<Transaction>> {
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
            .map_err(|e| {
                AptosErrorResponse::InternalServerError(Json(
                    AptosError::new(
                        format_err!("Failed to read raw transactions from storage: {}", e)
                            .to_string(),
                    )
                    .error_code(AptosErrorCode::InvalidBcsInStorageError),
                ))
            })?;

        self.render_transactions(data, accept_type, &latest_ledger_info)
    }

    fn render_transactions(
        &self,
        data: Vec<TransactionOnChainData>,
        accept_type: &AcceptType,
        latest_ledger_info: &LedgerInfo,
    ) -> AptosResponseResult<Vec<Transaction>> {
        if data.is_empty() {
            return AptosResponse::try_from_rust_value(vec![], latest_ledger_info, accept_type);
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
            .map_err(|e| {
                AptosErrorResponse::InternalServerError(Json(
                    AptosError::new(
                        format_err!("Failed to convert transaction data from storage: {}", e)
                            .to_string(),
                    )
                    .error_code(AptosErrorCode::InvalidBcsInStorageError),
                ))
            })?;

        AptosResponse::try_from_rust_value(txns, latest_ledger_info, accept_type)
    }
}
