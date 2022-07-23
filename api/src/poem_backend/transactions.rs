// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::accept_type::{parse_accept, AcceptType};
use super::bcs_payload::Bcs;
use super::page::Page;
use super::{
    ApiTags, AptosErrorResponse, BasicError, BasicErrorWith404, BasicResponse, BasicResponseStatus,
    BasicResult, BasicResultWith404, InternalError,
};
use super::{AptosErrorCode, BadRequestError, InsufficientStorageError};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::{generate_error_response, generate_success_response};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    AsConverter, EncodeSubmissionRequest, HexEncodedBytes, LedgerInfo, PendingTransaction,
    SubmitTransactionRequest, Transaction, TransactionOnChainData,
};
use aptos_crypto::signing_message;
use aptos_types::mempool_status::MempoolStatusCode;
use aptos_types::transaction::{RawTransaction, RawTransactionWithData, SignedTransaction};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{ApiRequest, OpenApi};

generate_success_response!(SubmitTransactionResponse, (202, Accepted));
generate_error_response!(
    SubmitTransactionError,
    (400, BadRequest),
    (413, PayloadTooLarge),
    (500, Internal),
    (507, InsufficientStorage)
);

type SubmitTransactionResult<T> =
    poem::Result<SubmitTransactionResponse<T>, SubmitTransactionError>;

// TODO: Consider making both content types accept either
// SubmitTransactionRequest or SignedTransaction (using AptosPost), the way
// it is now is quite confusing.

// We need a custom type here because we use different types for each of the
// content types possible for the POST data.
#[derive(ApiRequest)]
pub enum SubmitTransactionPost {
    // TODO: Consider just using the same BCS content type as usual, or that
    // with `+signed` on the end or something.
    // TODO: Switch from Vec<u8> to SignedTransaction. This has the benefit of
    // making Poem deserialize the data for us as well as describing the
    // expected input in the OpenAPI spec.
    #[oai(content_type = "application/x.aptos.signed_transaction+bcs")]
    Bcs(Bcs<Vec<u8>>),

    #[oai(content_type = "application/json")]
    Json(Json<SubmitTransactionRequest>),
}

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

    // TODO: Add custom sizelimit middleware.
    // See https://github.com/poem-web/poem/issues/331.
    //
    // TODO: The original endpoint is all kinds of weird. The spec says it
    // takes a SubmitTransactionRequest, but in reality it actually takes
    // just a UserTransactionRequest, there is no such thing as a SubmitTransactionRequest.
    // Really the SubmitTransactionRequest in the spec is what UserTransactionRequest
    // is, but UserTransactionRequest in the existing spec is missing fields that they
    // then add to SubmitTransactionRequest. Overall bizarre. I think we need to
    // make a new struct for UserTransactionRequest that doesn't have the
    // signature and flatten it in to a real SubmitTransactionRequest.
    // This is all for JSON. For BCS, it takes in bytes it expects to
    // deserialize into a SignedTransaction.
    // Make sense of all this.
    //
    /// Submit transaction
    ///
    /// todo
    #[oai(
        path = "/transactions",
        method = "post",
        operation_id = "submit_transaction",
        tag = "ApiTags::Transactions"
    )]
    async fn submit_transaction(
        &self,
        accept: Accept,
        data: SubmitTransactionPost,
    ) -> SubmitTransactionResult<PendingTransaction> {
        fail_point_poem("endppoint_submit_transaction")?;
        let accept_type = parse_accept(&accept)?;
        match data {
            SubmitTransactionPost::Bcs(data) => {
                let signed_transaction = bcs::from_bytes(&data)
                    .context("Failed to deserialize input into SignedTransaction")
                    .map_err(SubmitTransactionError::bad_request)?;
                self.create(&accept_type, signed_transaction).await
            }
            SubmitTransactionPost::Json(data) => {
                self.create_from_request(&accept_type, data.0).await
            }
        }
    }

    // TODO: The previous language around this endpoint used "signing message".
    // From what I can tell, all this endpoint is really doing is encoding the
    // request as BCS. To your average user (read: not knowledgable about
    // cryptography), "signing message" is needlessly confusing, hence the name
    // change. Discuss this further with the team.

    /// Encode submission
    ///
    /// This endpoint accepts an EncodeSubmissionRequest, which internally is a
    /// UserTransactionRequestInner (and optionally secondary signers) encoded
    /// as JSON, validates the request format, and then returns that request
    /// encoded in BCS. The client can then use this to create a transaction
    /// signature to be used in a SubmitTransactionRequest, which it then
    /// passes to the /transactions POST endpoint.
    ///
    /// To be clear, this endpoint makes it possible to submit transaction
    /// requests to the API from languages that do not have library support for
    /// BCS. If you are using an SDK that has BCS support, such as the official
    /// Rust, TypeScript, or Python SDKs, you do not need to use this endpoint.
    ///
    /// To sign a message using the response from this endpoint:
    /// - Decode the hex encoded string in the response to bytes.
    /// - Sign the bytes to create the signature.
    /// - Use that as the signature field in something like Ed25519Signature,
    ///   which you then use to build a TransactionSignature.
    ///
    /// TODO: Link an example of how to do this. Use externalDoc.
    #[oai(
        path = "/transactions/encode_submission",
        method = "post",
        operation_id = "encode_submission",
        tag = "ApiTags::Transactions"
    )]
    async fn encode_submission(
        &self,
        accept: Accept,
        data: Json<EncodeSubmissionRequest>,
        // TODO: Use a new request type that can't return 507.
    ) -> BasicResult<HexEncodedBytes> {
        fail_point_poem("endpoint_encode_submission")?;
        let accept_type = parse_accept(&accept)?;
        self.get_signing_message(&accept_type, data.0)
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

    async fn create_from_request(
        &self,
        accept_type: &AcceptType,
        req: SubmitTransactionRequest,
    ) -> SubmitTransactionResult<PendingTransaction> {
        let txn = self
            .context
            .move_resolver_poem()?
            .as_converter()
            .try_into_signed_transaction_poem(req, self.context.chain_id())
            .context("Failed to create SignedTransaction from SubmitTransactionRequest")
            .map_err(SubmitTransactionError::bad_request)?;
        self.create(accept_type, txn).await
    }

    async fn create(
        &self,
        accept_type: &AcceptType,
        txn: SignedTransaction,
    ) -> SubmitTransactionResult<PendingTransaction> {
        let ledger_info = self.context.get_latest_ledger_info_poem()?;
        let (mempool_status, vm_status_opt) = self
            .context
            .submit_transaction(txn.clone())
            .await
            .context("Mempool failed to initially evaluate submitted transaction")
            .map_err(SubmitTransactionError::internal)?;
        match mempool_status.code {
            MempoolStatusCode::Accepted => {
                let resolver = self.context.move_resolver_poem()?;
                let pending_txn = resolver
                    .as_converter()
                    .try_into_pending_transaction_poem(txn)
                    .context("Failed to build PendingTransaction from mempool response, even though it said the request was accepted")
                    .map_err(SubmitTransactionError::internal)?;
                SubmitTransactionResponse::try_from_rust_value((
                    pending_txn,
                    &ledger_info,
                    SubmitTransactionResponseStatus::Accepted,
                    accept_type,
                ))
            }
            MempoolStatusCode::MempoolIsFull => Err(
                SubmitTransactionError::insufficient_storage_str(&mempool_status.message),
            ),
            MempoolStatusCode::VmError => Err(SubmitTransactionError::bad_request_str(&format!(
                "invalid transaction: {}",
                vm_status_opt
                    .map(|s| format!("{:?}", s))
                    .unwrap_or_else(|| "UNKNOWN".to_owned())
            ))),
            _ => Err(SubmitTransactionError::bad_request_str(&format!(
                "transaction is rejected: {}",
                mempool_status,
            ))),
        }
    }

    pub fn get_signing_message(
        &self,
        accept_type: &AcceptType,
        request: EncodeSubmissionRequest,
    ) -> BasicResult<HexEncodedBytes> {
        let resolver = self.context.move_resolver_poem()?;
        let raw_txn: RawTransaction = resolver
            .as_converter()
            .try_into_raw_transaction_poem(request.transaction, self.context.chain_id())
            .context("The given transaction is invalid")
            .map_err(BasicError::bad_request)?;

        let raw_message = match request.secondary_signers {
            Some(secondary_signer_addresses) => {
                signing_message(&RawTransactionWithData::new_multi_agent(
                    raw_txn,
                    secondary_signer_addresses
                        .into_iter()
                        .map(|v| v.into())
                        .collect(),
                ))
            }
            None => raw_txn.signing_message(),
        };

        BasicResponse::try_from_rust_value((
            HexEncodedBytes::from(raw_message),
            // TODO: Make a variant that doesn't require ledger info.
            &self.context.get_latest_ledger_info_poem()?,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }
}
