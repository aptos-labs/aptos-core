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
    BasicResult, BasicResultWith404, InternalError, NotFoundError,
};
use super::{AptosErrorCode, BadRequestError, InsufficientStorageError};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::{generate_error_response, generate_success_response};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    Address, AsConverter, EncodeSubmissionRequest, HashValue, HexEncodedBytes, LedgerInfo,
    PendingTransaction, SubmitTransactionRequest, Transaction, TransactionData,
    TransactionOnChainData, U64,
};
use aptos_crypto::signing_message;
use aptos_types::mempool_status::MempoolStatusCode;
use aptos_types::transaction::{
    ExecutionStatus, RawTransaction, RawTransactionWithData, SignedTransaction, TransactionStatus,
};
use aptos_vm::AptosVM;
use poem::web::Accept;
use poem_openapi::param::{Path, Query};
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

type SimulateTransactionResult<T> = poem::Result<BasicResponse<T>, SubmitTransactionError>;

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

    /// Get transaction by hash
    ///
    /// todo
    #[oai(
        path = "/transactions/by_hash/:txn_hash",
        method = "get",
        operation_id = "get_transaction_by_hash",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transaction_by_hash(
        &self,
        accept: Accept,
        txn_hash: Path<HashValue>,
        // TODO: Use a new request type that can't return 507.
    ) -> BasicResultWith404<Transaction> {
        fail_point_poem("endpoint_transaction_by_hash")?;
        let accept_type = parse_accept(&accept)?;
        self.get_transaction_by_hash_inner(&accept_type, txn_hash.0)
            .await
    }

    /// Get transaction by version
    ///
    /// todo
    #[oai(
        path = "/transactions/by_version/:txn_version",
        method = "get",
        operation_id = "get_transaction_by_version",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transaction_by_version(
        &self,
        accept: Accept,
        txn_version: Path<U64>,
        // TODO: Use a new request type that can't return 507.
    ) -> BasicResultWith404<Transaction> {
        fail_point_poem("endpoint_transaction_by_version")?;
        let accept_type = parse_accept(&accept)?;
        self.get_transaction_by_version_inner(&accept_type, txn_version.0)
            .await
    }

    /// Get account transactions
    ///
    /// todo
    /// todo, note that this currently returns [] even if the account doesn't
    /// exist, when it should really return a 404.
    #[oai(
        path = "/accounts/:address/transactions",
        method = "get",
        operation_id = "get_account_transactions",
        tag = "ApiTags::Transactions"
    )]
    async fn get_accounts_transactions(
        &self,
        accept: Accept,
        address: Path<Address>,
        start: Query<Option<u64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Transaction>> {
        fail_point_poem("endpoint_get_accounts_transactions")?;
        let accept_type = parse_accept(&accept)?;
        let page = Page::new(start.0, limit.0);
        self.list_by_account(&accept_type, page, address.0)
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
        fail_point_poem("endpoint_submit_transaction")?;
        let accept_type = parse_accept(&accept)?;
        let signed_transaction = self.get_signed_transaction(data)?;
        self.create(&accept_type, signed_transaction).await
    }

    /// Simulate transaction
    ///
    /// Simulate submitting a transaction. To use this, you must:
    /// - Create a SignedTransaction with a zero-padded signature.
    /// - Submit a SubmitTransactionRequest containing a UserTransactionRequest containing that signature.
    #[oai(
        path = "/transactions/simulate",
        method = "post",
        operation_id = "simulate_transaction",
        tag = "ApiTags::Transactions"
    )]
    async fn simulate_transaction(
        &self,
        accept: Accept,
        data: SubmitTransactionPost,
    ) -> SimulateTransactionResult<Vec<Transaction>> {
        fail_point_poem("endpoint_simulate_transaction")?;
        let accept_type = parse_accept(&accept)?;
        let signed_transaction = self.get_signed_transaction(data)?;
        self.simulate(&accept_type, signed_transaction).await
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
    /// - Use that as the signature field in something like Ed25519Signature, which you then use to build a TransactionSignature.
    //
    // TODO: Link an example of how to do this. Use externalDoc.
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
        // TODO: Use a new request type that can't return 507 but still returns all the other necessary errors.
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
        // TODO: I think there is a bug at startup where limit 1 still returns
        // every transaction.
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

    fn render_transactions<E: InternalError>(
        &self,
        data: Vec<TransactionOnChainData>,
        accept_type: &AcceptType,
        latest_ledger_info: &LedgerInfo,
    ) -> Result<BasicResponse<Vec<Transaction>>, E> {
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
            .map_err(E::internal)?;

        BasicResponse::try_from_rust_value((
            txns,
            latest_ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }

    async fn get_transaction_by_hash_inner(
        &self,
        accept_type: &AcceptType,
        hash: HashValue,
    ) -> BasicResultWith404<Transaction> {
        let ledger_info = self.context.get_latest_ledger_info_poem()?;
        let txn_data = self
            .get_by_hash(hash.into(), &ledger_info)
            .await
            .context(format!("Failed to get transaction by hash {}", hash))
            // TODO: Should this be a 500?
            .map_err(BasicErrorWith404::not_found)?
            .context(format!("Failed to find transaction with hash: {}", hash))
            .map_err(BasicErrorWith404::not_found)?;

        self.get_transaction_inner(accept_type, txn_data, &ledger_info)
            .await
    }

    async fn get_transaction_by_version_inner(
        &self,
        accept_type: &AcceptType,
        version: U64,
    ) -> BasicResultWith404<Transaction> {
        let ledger_info = self.context.get_latest_ledger_info_poem()?;
        let txn_data = self
            .get_by_version(version.0, &ledger_info)
            .context(format!("Failed to get transaction by version {}", version))
            // TODO: Should this be a 500?
            .map_err(BasicErrorWith404::not_found)?
            .context(format!(
                "Failed to find transaction at version: {}",
                version
            ))
            .map_err(BasicErrorWith404::not_found)?;

        self.get_transaction_inner(accept_type, txn_data, &ledger_info)
            .await
    }

    async fn get_transaction_inner(
        &self,
        accept_type: &AcceptType,
        transaction_data: TransactionData,
        ledger_info: &LedgerInfo,
    ) -> BasicResultWith404<Transaction> {
        let resolver = self.context.move_resolver_poem()?;
        let transaction = match transaction_data {
            TransactionData::OnChain(txn) => {
                let timestamp = self
                    .context
                    .get_block_timestamp(txn.version)
                    .context("Failed to get block timestamp from DB")
                    .map_err(BasicErrorWith404::internal)?;
                resolver
                    .as_converter()
                    .try_into_onchain_transaction(timestamp, txn)
                    .context("Failed to convert on chain transaction to Transaction")
                    .map_err(BasicErrorWith404::internal)?
            }
            TransactionData::Pending(txn) => resolver
                .as_converter()
                .try_into_pending_transaction(*txn)
                .context("Failed to convert on pending transaction to Transaction")
                .map_err(BasicErrorWith404::internal)?,
        };

        BasicResponse::try_from_rust_value((
            transaction,
            ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }

    fn get_by_version(
        &self,
        version: u64,
        ledger_info: &LedgerInfo,
    ) -> anyhow::Result<Option<TransactionData>> {
        if version > ledger_info.version() {
            return Ok(None);
        }
        Ok(Some(
            self.context
                .get_transaction_by_version(version, ledger_info.version())?
                .into(),
        ))
    }

    // This function looks for the transaction by hash in database and then mempool,
    // because the period a transaction stay in the mempool is likely short.
    // Although the mempool get transation is async, but looking up txn in database is a sync call,
    // thus we keep it simple and call them in sequence.
    async fn get_by_hash(
        &self,
        hash: aptos_crypto::HashValue,
        ledger_info: &LedgerInfo,
    ) -> anyhow::Result<Option<TransactionData>> {
        let from_db = self
            .context
            .get_transaction_by_hash(hash, ledger_info.version())?;
        Ok(match from_db {
            None => self
                .context
                .get_pending_transaction_by_hash(hash)
                .await?
                .map(|t| t.into()),
            _ => from_db.map(|t| t.into()),
        })
    }

    fn list_by_account(
        &self,
        accept_type: &AcceptType,
        page: Page,
        address: Address,
    ) -> BasicResultWith404<Vec<Transaction>> {
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        // TODO: Return more specific errors from within this function.
        let data = self
            .context
            .get_account_transactions(
                address.into(),
                page.start(0, u64::MAX)?,
                page.limit()?,
                latest_ledger_info.version(),
            )
            .context("Failed to get account transactions for the given account")
            .map_err(BasicErrorWith404::internal)?;

        self.render_transactions(data, accept_type, &latest_ledger_info)
    }

    fn get_signed_transaction(
        &self,
        data: SubmitTransactionPost,
    ) -> Result<SignedTransaction, SubmitTransactionError> {
        match data {
            SubmitTransactionPost::Bcs(data) => {
                let signed_transaction = bcs::from_bytes(&data)
                    .context("Failed to deserialize input into SignedTransaction")
                    .map_err(SubmitTransactionError::bad_request)?;
                Ok(signed_transaction)
            }
            SubmitTransactionPost::Json(data) => self
                .context
                .move_resolver_poem()?
                .as_converter()
                .try_into_signed_transaction_poem(data.0, self.context.chain_id())
                .context("Failed to create SignedTransaction from SubmitTransactionRequest")
                .map_err(SubmitTransactionError::bad_request),
        }
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

    // TODO: This returns a Vec<Transaction>, but is it possible for a single
    // transaction request to result in multiple executed transactions?
    // TODO: Look at just returning (a) UserTransaction, since that's all users
    // are allowed to submit in the first place.
    // TODO: This function leverages a lot of types from aptos_types, use the
    // local API types and just return those directly, instead of converting
    // from these types in render_transactions.
    pub async fn simulate(
        &self,
        accept_type: &AcceptType,
        txn: SignedTransaction,
    ) -> SimulateTransactionResult<Vec<Transaction>> {
        if txn.clone().check_signature().is_ok() {
            return Err(SubmitTransactionError::bad_request_str(
                "Transaction simulation request has a valid signature, this is not allowed",
            ));
        }
        let ledger_info = self.context.get_latest_ledger_info_poem()?;
        let move_resolver = self.context.move_resolver_poem()?;
        let (status, output) = AptosVM::simulate_signed_transaction(&txn, &move_resolver);
        let version = ledger_info.version();
        let exe_status = match status.into() {
            TransactionStatus::Keep(exec_status) => exec_status,
            _ => ExecutionStatus::MiscellaneousError(None),
        };
        let zero_hash = aptos_crypto::HashValue::zero();
        let info = aptos_types::transaction::TransactionInfo::new(
            zero_hash,
            zero_hash,
            zero_hash,
            None,
            output.gas_used(),
            exe_status,
        );
        let simulated_txn = TransactionOnChainData {
            version,
            transaction: aptos_types::transaction::Transaction::UserTransaction(txn),
            info,
            events: output.events().to_vec(),
            accumulator_root_hash: aptos_crypto::HashValue::default(),
            changes: output.write_set().clone(),
        };

        self.render_transactions(vec![simulated_txn], accept_type, &ledger_info)
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
