// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    accounts::Account,
    bcs_payload::Bcs,
    context::{api_spawn_blocking, Context, FunctionStats},
    failpoint::fail_point_poem,
    generate_error_response, generate_success_response, metrics,
    metrics::WAIT_TRANSACTION_GAUGE,
    page::Page,
    response::{
        api_disabled, api_forbidden, transaction_not_found_by_hash,
        transaction_not_found_by_version, version_pruned, BadRequestError, BasicError,
        BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResult, BasicResultWith404,
        ForbiddenError, InsufficientStorageError, InternalError,
    },
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    verify_function_identifier, verify_module_identifier, Address, AptosError, AptosErrorCode,
    AsConverter, EncodeSubmissionRequest, GasEstimation, GasEstimationBcs, HashValue,
    HexEncodedBytes, LedgerInfo, MoveType, PendingTransaction, SubmitTransactionRequest,
    Transaction, TransactionData, TransactionOnChainData, TransactionsBatchSingleSubmissionFailure,
    TransactionsBatchSubmissionResult, UserTransaction, VerifyInput, VerifyInputWithRecursion,
    MAX_RECURSIVE_TYPES_ALLOWED, U64,
};
use aptos_crypto::{hash::CryptoHash, signing_message};
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::MempoolStatusCode,
    transaction::{
        EntryFunction, ExecutionStatus, MultisigTransactionPayload, RawTransaction,
        RawTransactionWithData, SignedTransaction, TransactionPayload,
    },
    vm_status::StatusCode,
    AptosCoinType, CoinType,
};
use aptos_vm::{AptosSimulationVM, AptosVM};
use move_core_types::{ident_str, language_storage::ModuleId, vm_status::VMStatus};
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    ApiRequest, OpenApi,
};
use std::{sync::Arc, time::Duration};

generate_success_response!(SubmitTransactionResponse, (202, Accepted));

generate_error_response!(
    SubmitTransactionError,
    (400, BadRequest),
    (403, Forbidden),
    (404, NotFound),
    (413, PayloadTooLarge),
    (500, Internal),
    (503, ServiceUnavailable),
    (507, InsufficientStorage)
);

type SubmitTransactionResult<T> =
    poem::Result<SubmitTransactionResponse<T>, SubmitTransactionError>;

generate_success_response!(
    SubmitTransactionsBatchResponse,
    (202, Accepted),
    (206, AcceptedPartial)
);

type SubmitTransactionsBatchResult<T> =
    poem::Result<SubmitTransactionsBatchResponse<T>, SubmitTransactionError>;

type SimulateTransactionResult<T> = poem::Result<BasicResponse<T>, SubmitTransactionError>;

// TODO: Consider making both content types accept either
// SubmitTransactionRequest or SignedTransaction, the way
// it is now is quite confusing.

// We need a custom type here because we use different types for each of the
// content types possible for the POST data.
#[derive(ApiRequest, Debug)]
pub enum SubmitTransactionPost {
    #[oai(content_type = "application/json")]
    Json(Json<SubmitTransactionRequest>),

    // TODO: Since I don't want to impl all the Poem derives on SignedTransaction,
    // find a way to at least indicate in the spec that it expects a SignedTransaction.
    // TODO: https://github.com/aptos-labs/aptos-core/issues/2275
    #[oai(content_type = "application/x.aptos.signed_transaction+bcs")]
    Bcs(Bcs),
}

impl VerifyInput for SubmitTransactionPost {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            SubmitTransactionPost::Json(inner) => inner.0.verify(),
            SubmitTransactionPost::Bcs(_) => Ok(()),
        }
    }
}

// We need a custom type here because we use different types for each of the
// content types possible for the POST data.
#[derive(ApiRequest, Debug)]
pub enum SubmitTransactionsBatchPost {
    #[oai(content_type = "application/json")]
    Json(Json<Vec<SubmitTransactionRequest>>),

    // TODO: Since I don't want to impl all the Poem derives on SignedTransaction,
    // find a way to at least indicate in the spec that it expects a SignedTransaction.
    // TODO: https://github.com/aptos-labs/aptos-core/issues/2275
    #[oai(content_type = "application/x.aptos.signed_transaction+bcs")]
    Bcs(Bcs),
}

impl VerifyInput for SubmitTransactionsBatchPost {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            SubmitTransactionsBatchPost::Json(inner) => {
                for request in inner.0.iter() {
                    request.verify()?;
                }
            },
            SubmitTransactionsBatchPost::Bcs(_) => {},
        }
        Ok(())
    }
}

/// API for interacting with transactions
#[derive(Clone)]
pub struct TransactionsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl TransactionsApi {
    /// Get transactions
    ///
    /// Retrieve on-chain committed transactions. The page size and start ledger version
    /// can be provided to get a specific sequence of transactions.
    ///
    /// If the version has been pruned, then a 410 will be returned.
    ///
    /// To retrieve a pending transaction, use /transactions/by_hash.
    #[oai(
        path = "/transactions",
        method = "get",
        operation_id = "get_transactions",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transactions(
        &self,
        accept_type: AcceptType,
        /// Ledger version to start list of transactions
        ///
        /// If not provided, defaults to showing the latest transactions
        start: Query<Option<U64>>,
        /// Max number of transactions to retrieve.
        ///
        /// If not provided, defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Transaction>> {
        fail_point_poem("endpoint_get_transactions")?;
        self.context
            .check_api_output_enabled("Get transactions", &accept_type)?;
        let page = Page::new(
            start.0.map(|v| v.0),
            limit.0,
            self.context.max_transactions_page_size(),
        );

        let api = self.clone();
        api_spawn_blocking(move || api.list(&accept_type, page)).await
    }

    /// Get transaction by hash
    ///
    /// Look up a transaction by its hash. This is the same hash that is returned
    /// by the API when submitting a transaction (see PendingTransaction).
    ///
    /// When given a transaction hash, the server first looks for the transaction
    /// in storage (on-chain, committed). If no on-chain transaction is found, it
    /// looks the transaction up by hash in the mempool (pending, not yet committed).
    ///
    /// To create a transaction hash by yourself, do the following:
    ///   1. Hash message bytes: "RawTransaction" bytes + BCS bytes of [Transaction](https://aptos-labs.github.io/aptos-core/aptos_types/transaction/enum.Transaction.html).
    ///   2. Apply hash algorithm `SHA3-256` to the hash message bytes.
    ///   3. Hex-encode the hash bytes with `0x` prefix.
    // TODO: Include a link to an example of how to do this ^
    #[oai(
        path = "/transactions/by_hash/:txn_hash",
        method = "get",
        operation_id = "get_transaction_by_hash",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transaction_by_hash(
        &self,
        accept_type: AcceptType,
        /// Hash of transaction to retrieve
        txn_hash: Path<HashValue>,
        // TODO: Use a new request type that can't return 507.
    ) -> BasicResultWith404<Transaction> {
        fail_point_poem("endpoint_transaction_by_hash")?;
        self.context
            .check_api_output_enabled("Get transactions by hash", &accept_type)?;
        self.get_transaction_by_hash_inner(&accept_type, txn_hash.0)
            .await
    }

    /// Wait for transaction by hash
    ///
    /// Same as /transactions/by_hash, but will wait for a pending transaction to be committed. To be used as a long
    /// poll optimization by clients, to reduce latency caused by polling. The "long" poll is generally a second or
    /// less but dictated by the server; the client must deal with the result as if the request was a normal
    /// /transactions/by_hash request, e.g., by retrying if the transaction is pending.
    #[oai(
        path = "/transactions/wait_by_hash/:txn_hash",
        method = "get",
        operation_id = "wait_transaction_by_hash",
        tag = "ApiTags::Transactions"
    )]
    async fn wait_transaction_by_hash(
        &self,
        accept_type: AcceptType,
        /// Hash of transaction to retrieve
        txn_hash: Path<HashValue>,
        // TODO: Use a new request type that can't return 507.
    ) -> BasicResultWith404<Transaction> {
        fail_point_poem("endpoint_wait_transaction_by_hash")?;
        self.context
            .check_api_output_enabled("Get transactions by hash", &accept_type)?;

        // Short poll if the active connections are too high
        if self
            .context
            .wait_for_hash_active_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            >= self
                .context
                .node_config
                .api
                .wait_by_hash_max_active_connections
        {
            self.context
                .wait_for_hash_active_connections
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
            metrics::WAIT_TRANSACTION_POLL_TIME
                .with_label_values(&["short"])
                .observe(0.0);
            return self
                .get_transaction_by_hash_inner(&accept_type, txn_hash.0)
                .await;
        }

        let start_time = std::time::Instant::now();
        WAIT_TRANSACTION_GAUGE.inc();

        let result = self
            .wait_transaction_by_hash_inner(
                &accept_type,
                txn_hash.0,
                self.context.node_config.api.wait_by_hash_timeout_ms,
                self.context.node_config.api.wait_by_hash_poll_interval_ms,
            )
            .await;

        WAIT_TRANSACTION_GAUGE.dec();
        self.context
            .wait_for_hash_active_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        metrics::WAIT_TRANSACTION_POLL_TIME
            .with_label_values(&["long"])
            .observe(start_time.elapsed().as_secs_f64());
        result
    }

    /// Get transaction by version
    ///
    /// Retrieves a transaction by a given version. If the version has been
    /// pruned, a 410 will be returned.
    #[oai(
        path = "/transactions/by_version/:txn_version",
        method = "get",
        operation_id = "get_transaction_by_version",
        tag = "ApiTags::Transactions"
    )]
    async fn get_transaction_by_version(
        &self,
        accept_type: AcceptType,
        /// Version of transaction to retrieve
        txn_version: Path<U64>,
    ) -> BasicResultWith404<Transaction> {
        fail_point_poem("endpoint_transaction_by_version")?;
        self.context
            .check_api_output_enabled("Get transactions by version", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.get_transaction_by_version_inner(&accept_type, txn_version.0)
        })
        .await
    }

    /// Get account transactions
    ///
    /// Retrieves on-chain committed transactions from an account. If the start
    /// version is too far in the past, a 410 will be returned.
    ///
    /// If no start version is given, it will start at version 0.
    ///
    /// To retrieve a pending transaction, use /transactions/by_hash.
    #[oai(
        path = "/accounts/:address/transactions",
        method = "get",
        operation_id = "get_account_transactions",
        tag = "ApiTags::Transactions"
    )]
    async fn get_accounts_transactions(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Account sequence number to start list of transactions
        ///
        /// If not provided, defaults to showing the latest transactions
        start: Query<Option<U64>>,
        /// Max number of transactions to retrieve.
        ///
        /// If not provided, defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Transaction>> {
        fail_point_poem("endpoint_get_accounts_transactions")?;
        self.context
            .check_api_output_enabled("Get account transactions", &accept_type)?;
        let page = Page::new(
            start.0.map(|v| v.0),
            limit.0,
            self.context.max_transactions_page_size(),
        );
        let api = self.clone();
        api_spawn_blocking(move || api.list_by_account(&accept_type, page, address.0)).await
    }

    /// Submit transaction
    ///
    /// This endpoint accepts transaction submissions in two formats.
    ///
    /// To submit a transaction as JSON, you must submit a SubmitTransactionRequest.
    /// To build this request, do the following:
    ///
    ///   1. Encode the transaction as BCS. If you are using a language that has
    ///      native BCS support, make sure of that library. If not, you may take
    ///      advantage of /transactions/encode_submission. When using this
    ///      endpoint, make sure you trust the node you're talking to, as it is
    ///      possible they could manipulate your request.
    ///   2. Sign the encoded transaction and use it to create a TransactionSignature.
    ///   3. Submit the request. Make sure to use the "application/json" Content-Type.
    ///
    /// To submit a transaction as BCS, you must submit a SignedTransaction
    /// encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
    /// Make sure to use the `application/x.aptos.signed_transaction+bcs` Content-Type.
    // TODO: Point to examples of both of these flows, in multiple languages.
    #[oai(
        path = "/transactions",
        method = "post",
        operation_id = "submit_transaction",
        tag = "ApiTags::Transactions"
    )]
    async fn submit_transaction(
        &self,
        accept_type: AcceptType,
        data: SubmitTransactionPost,
    ) -> SubmitTransactionResult<PendingTransaction> {
        data.verify()
            .context("Submitted transaction invalid'")
            .map_err(|err| {
                SubmitTransactionError::bad_request_with_code_no_info(
                    err,
                    AptosErrorCode::InvalidInput,
                )
            })?;
        fail_point_poem("endpoint_submit_transaction")?;
        if !self.context.node_config.api.transaction_submission_enabled {
            return Err(api_disabled("Submit transaction"));
        }
        self.context
            .check_api_output_enabled("Submit transaction", &accept_type)?;
        let ledger_info = self.context.get_latest_ledger_info()?;
        let signed_transaction = self.get_signed_transaction(&ledger_info, data)?;
        self.create(&accept_type, &ledger_info, signed_transaction)
            .await
    }

    /// Submit batch transactions
    ///
    /// This allows you to submit multiple transactions.  The response has three outcomes:
    ///
    ///   1. All transactions succeed, and it will return a 202
    ///   2. Some transactions succeed, and it will return the failed transactions and a 206
    ///   3. No transactions succeed, and it will also return the failed transactions and a 206
    ///
    /// To submit a transaction as JSON, you must submit a SubmitTransactionRequest.
    /// To build this request, do the following:
    ///
    ///   1. Encode the transaction as BCS. If you are using a language that has
    ///      native BCS support, make sure to use that library. If not, you may take
    ///      advantage of /transactions/encode_submission. When using this
    ///      endpoint, make sure you trust the node you're talking to, as it is
    ///      possible they could manipulate your request.
    ///   2. Sign the encoded transaction and use it to create a TransactionSignature.
    ///   3. Submit the request. Make sure to use the "application/json" Content-Type.
    ///
    /// To submit a transaction as BCS, you must submit a SignedTransaction
    /// encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
    /// Make sure to use the `application/x.aptos.signed_transaction+bcs` Content-Type.
    #[oai(
        path = "/transactions/batch",
        method = "post",
        operation_id = "submit_batch_transactions",
        tag = "ApiTags::Transactions"
    )]
    async fn submit_transactions_batch(
        &self,
        accept_type: AcceptType,
        data: SubmitTransactionsBatchPost,
    ) -> SubmitTransactionsBatchResult<TransactionsBatchSubmissionResult> {
        data.verify()
            .context("Submitted transactions invalid")
            .map_err(|err| {
                SubmitTransactionError::bad_request_with_code_no_info(
                    err,
                    AptosErrorCode::InvalidInput,
                )
            })?;
        fail_point_poem("endpoint_submit_batch_transactions")?;
        if !self.context.node_config.api.transaction_submission_enabled {
            return Err(api_disabled("Submit batch transaction"));
        }
        self.context
            .check_api_output_enabled("Submit batch transactions", &accept_type)?;
        let ledger_info = self.context.get_latest_ledger_info()?;
        let signed_transactions_batch = self.get_signed_transactions_batch(&ledger_info, data)?;
        if self.context.max_submit_transaction_batch_size() < signed_transactions_batch.len() {
            return Err(SubmitTransactionError::bad_request_with_code(
                format!(
                    "Submitted too many transactions: {}, while limit is {}",
                    signed_transactions_batch.len(),
                    self.context.max_submit_transaction_batch_size(),
                ),
                AptosErrorCode::InvalidInput,
                &ledger_info,
            ));
        }
        self.create_batch(&accept_type, &ledger_info, signed_transactions_batch)
            .await
    }

    /// Simulate transaction
    ///
    /// The output of the transaction will have the exact transaction outputs and events that running
    /// an actual signed transaction would have.  However, it will not have the associated state
    /// hashes, as they are not updated in storage.  This can be used to estimate the maximum gas
    /// units for a submitted transaction.
    ///
    /// To use this, you must:
    /// - Create a SignedTransaction with a zero-padded signature.
    /// - Submit a SubmitTransactionRequest containing a UserTransactionRequest containing that signature.
    ///
    /// To use this endpoint with BCS, you must submit a SignedTransaction
    /// encoded as BCS. See SignedTransaction in types/src/transaction/mod.rs.
    #[oai(
        path = "/transactions/simulate",
        method = "post",
        operation_id = "simulate_transaction",
        tag = "ApiTags::Transactions"
    )]
    async fn simulate_transaction(
        &self,
        accept_type: AcceptType,
        /// If set to true, the max gas value in the transaction will be ignored
        /// and the maximum possible gas will be used
        estimate_max_gas_amount: Query<Option<bool>>,
        /// If set to true, the gas unit price in the transaction will be ignored
        /// and the estimated value will be used
        estimate_gas_unit_price: Query<Option<bool>>,
        /// If set to true, the transaction will use a higher price than the original
        /// estimate.
        estimate_prioritized_gas_unit_price: Query<Option<bool>>,
        data: SubmitTransactionPost,
    ) -> SimulateTransactionResult<Vec<UserTransaction>> {
        data.verify()
            .context("Simulated transaction invalid")
            .map_err(|err| {
                SubmitTransactionError::bad_request_with_code_no_info(
                    err,
                    AptosErrorCode::InvalidInput,
                )
            })?;
        fail_point_poem("endpoint_simulate_transaction")?;
        if !self.context.node_config.api.transaction_simulation_enabled {
            return Err(api_disabled("Simulate transaction"));
        }
        self.context
            .check_api_output_enabled("Simulate transaction", &accept_type)?;

        let api = self.clone();
        let context = self.context.clone();
        api_spawn_blocking(move || {
            let ledger_info = context.get_latest_ledger_info()?;
            let mut signed_transaction = api.get_signed_transaction(&ledger_info, data)?;

            // Confirm the simulation filter allows the transaction. We use HashValue::zero()
            // here for the block ID because we don't allow filtering by block ID for the
            // simulation filters. See the ConfigSanitizer for ApiConfig.
            if !context.node_config.api.simulation_filter.allows(
                aptos_crypto::HashValue::zero(),
                ledger_info.timestamp(),
                &signed_transaction,
            ) {
                return Err(SubmitTransactionError::forbidden_with_code(
                    "Transaction not allowed by simulation filter",
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                ));
            }

            let estimated_gas_unit_price = match (
                estimate_gas_unit_price.0.unwrap_or_default(),
                estimate_prioritized_gas_unit_price.0.unwrap_or_default(),
            ) {
                (_, true) => {
                    let gas_estimation = context.estimate_gas_price(&ledger_info)?;
                    // The prioritized gas estimate should always be set, but if it's not use the gas estimate
                    Some(
                        gas_estimation
                            .prioritized_gas_estimate
                            .unwrap_or(gas_estimation.gas_estimate),
                    )
                },
                (true, false) => Some(context.estimate_gas_price(&ledger_info)?.gas_estimate),
                (false, false) => None,
            };

            // If estimate max gas amount is provided, we will just make it the maximum value
            let estimated_max_gas_amount = if estimate_max_gas_amount.0.unwrap_or_default() {
                // Retrieve max possible gas units
                let (_, gas_params) = context.get_gas_schedule(&ledger_info)?;
                let min_number_of_gas_units =
                    u64::from(gas_params.vm.txn.min_transaction_gas_units)
                        / u64::from(gas_params.vm.txn.gas_unit_scaling_factor);
                let max_number_of_gas_units =
                    u64::from(gas_params.vm.txn.maximum_number_of_gas_units);

                // Retrieve account balance to determine max gas available, right now this is using
                // a view function, but we may want to re-evaluate this based on performance
                let (_, _, state_view) = context
                    .state_view::<BasicErrorWith404>(Option::None)
                    .map_err(|err| {
                        SubmitTransactionError::bad_request_with_code_no_info(
                            err,
                            AptosErrorCode::InvalidInput,
                        )
                    })?;
                let output = AptosVM::execute_view_function(
                    &state_view,
                    ModuleId::new(AccountAddress::ONE, ident_str!("coin").into()),
                    ident_str!("balance").into(),
                    vec![AptosCoinType::type_tag()],
                    vec![signed_transaction.sender().to_vec()],
                    context.node_config.api.max_gas_view_function,
                );
                let values = output.values.map_err(|err| {
                    SubmitTransactionError::bad_request_with_code_no_info(
                        err,
                        AptosErrorCode::InvalidInput,
                    )
                })?;
                let balance: u64 = bcs::from_bytes(&values[0]).map_err(|err| {
                    SubmitTransactionError::bad_request_with_code_no_info(
                        err,
                        AptosErrorCode::InvalidInput,
                    )
                })?;

                let gas_unit_price =
                    estimated_gas_unit_price.unwrap_or_else(|| signed_transaction.gas_unit_price());

                // With 0 gas price, we set it to max gas units, since we can't divide by 0
                let max_account_gas_units = if gas_unit_price == 0 {
                    balance
                } else {
                    balance / gas_unit_price
                };

                // To give better error messaging, we should not go below the minimum number of gas units
                let max_account_gas_units =
                    std::cmp::max(min_number_of_gas_units, max_account_gas_units);

                // Minimum of the max account and the max total needs to be used for estimation
                Some(std::cmp::min(
                    max_account_gas_units,
                    max_number_of_gas_units,
                ))
            } else {
                None
            };

            // If there is an estimation of either, replace the values
            if estimated_max_gas_amount.is_some() || estimated_gas_unit_price.is_some() {
                signed_transaction = override_gas_parameters(
                    &signed_transaction,
                    estimated_max_gas_amount,
                    estimated_gas_unit_price,
                );
            }

            api.simulate(&accept_type, ledger_info, signed_transaction)
        })
        .await
    }

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
    #[oai(
        path = "/transactions/encode_submission",
        method = "post",
        operation_id = "encode_submission",
        tag = "ApiTags::Transactions"
    )]
    async fn encode_submission(
        &self,
        accept_type: AcceptType,
        data: Json<EncodeSubmissionRequest>,
        // TODO: Use a new request type that can't return 507 but still returns all the other necessary errors.
    ) -> BasicResult<HexEncodedBytes> {
        data.0
            .verify()
            .context("'UserTransactionRequest' invalid")
            .map_err(|err| {
                BasicError::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_encode_submission")?;
        if !self.context.node_config.api.encode_submission_enabled {
            return Err(api_forbidden(
                "Encode submission",
                "Only JSON is supported as an AcceptType.",
            ));
        }
        self.context
            .check_api_output_enabled("Encode submission", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || api.get_signing_message(&accept_type, data.0)).await
    }

    pub fn log_gas_estimation(gas_estimation: &GasEstimation) {
        metrics::GAS_ESTIMATE
            .with_label_values(&[metrics::GAS_ESTIMATE_CURRENT])
            .observe(gas_estimation.gas_estimate as f64);
        if let Some(deprioritized) = gas_estimation.deprioritized_gas_estimate {
            metrics::GAS_ESTIMATE
                .with_label_values(&[metrics::GAS_ESTIMATE_DEPRIORITIZED])
                .observe(deprioritized as f64);
        }
        if let Some(prioritized) = gas_estimation.prioritized_gas_estimate {
            metrics::GAS_ESTIMATE
                .with_label_values(&[metrics::GAS_ESTIMATE_PRIORITIZED])
                .observe(prioritized as f64);
        }
    }

    /// Estimate gas price
    ///
    /// Gives an estimate of the gas unit price required to get a transaction on chain in a
    /// reasonable amount of time. The gas unit price is the amount that each transaction commits to
    /// pay for each unit of gas consumed in executing the transaction. The estimate is based on
    /// recent history: it gives the minimum gas that would have been required to get into recent
    /// blocks, for blocks that were full. (When blocks are not full, the estimate will match the
    /// minimum gas unit price.)
    ///
    /// The estimation is given in three values: de-prioritized (low), regular, and prioritized
    /// (aggressive). Using a more aggressive value increases the likelihood that the transaction
    /// will make it into the next block; more aggressive values are computed with a larger history
    /// and higher percentile statistics. More details are in AIP-34.
    #[oai(
        path = "/estimate_gas_price",
        method = "get",
        operation_id = "estimate_gas_price",
        tag = "ApiTags::Transactions"
    )]
    async fn estimate_gas_price(&self, accept_type: AcceptType) -> BasicResult<GasEstimation> {
        fail_point_poem("endpoint_encode_submission")?;
        self.context
            .check_api_output_enabled("Estimate gas price", &accept_type)?;

        let context = self.context.clone();
        api_spawn_blocking(move || {
            let latest_ledger_info = context.get_latest_ledger_info()?;
            let gas_estimation = context.estimate_gas_price(&latest_ledger_info)?;
            Self::log_gas_estimation(&gas_estimation);

            match accept_type {
                AcceptType::Json => BasicResponse::try_from_json((
                    gas_estimation,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                )),
                AcceptType::Bcs => {
                    let gas_estimation_bcs = GasEstimationBcs {
                        gas_estimate: gas_estimation.gas_estimate,
                    };
                    BasicResponse::try_from_bcs((
                        gas_estimation_bcs,
                        &latest_ledger_info,
                        BasicResponseStatus::Ok,
                    ))
                },
            }
        })
        .await
    }
}

impl TransactionsApi {
    /// List all transactions paging by ledger version
    fn list(&self, accept_type: &AcceptType, page: Page) -> BasicResultWith404<Vec<Transaction>> {
        let latest_ledger_info = self.context.get_latest_ledger_info()?;
        let ledger_version = latest_ledger_info.version();

        let limit = page.limit(&latest_ledger_info)?;
        let start_version = page.compute_start(limit, ledger_version, &latest_ledger_info)?;
        let data = self
            .context
            .get_transactions(start_version, limit, ledger_version)
            .context("Failed to read raw transactions from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                let timestamp = self
                    .context
                    .get_block_timestamp(&latest_ledger_info, start_version)?;
                BasicResponse::try_from_json((
                    self.context.render_transactions_sequential(
                        &latest_ledger_info,
                        data,
                        timestamp,
                    )?,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            },
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((data, &latest_ledger_info, BasicResponseStatus::Ok))
            },
        }
    }

    async fn wait_transaction_by_hash_inner(
        &self,
        accept_type: &AcceptType,
        hash: HashValue,
        wait_by_hash_timeout_ms: u64,
        wait_by_hash_poll_interval_ms: u64,
    ) -> BasicResultWith404<Transaction> {
        let start_time = std::time::Instant::now();
        loop {
            let context = self.context.clone();
            let accept_type = accept_type.clone();

            let (internal_ledger_info_opt, storage_ledger_info) =
                api_spawn_blocking(move || context.get_latest_internal_and_storage_ledger_info())
                    .await?;
            let storage_version = storage_ledger_info.ledger_version.into();
            let internal_ledger_version = internal_ledger_info_opt
                .as_ref()
                .map(|info| info.ledger_version.into());
            let latest_ledger_info = internal_ledger_info_opt.unwrap_or(storage_ledger_info);
            let txn_data = self
                .get_by_hash(hash.into(), storage_version, internal_ledger_version)
                .await
                .context(format!("Failed to get transaction by hash {}", hash))
                .map_err(|err| {
                    BasicErrorWith404::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &latest_ledger_info,
                    )
                })?
                .context(format!("Failed to find transaction with hash: {}", hash))
                .map_err(|_| transaction_not_found_by_hash(hash, &latest_ledger_info))?;

            if matches!(txn_data, TransactionData::Pending(_))
                && (start_time.elapsed().as_millis() as u64) < wait_by_hash_timeout_ms
            {
                tokio::time::sleep(Duration::from_millis(wait_by_hash_poll_interval_ms)).await;
                continue;
            }

            let api = self.clone();
            return api_spawn_blocking(move || {
                api.get_transaction_inner(&accept_type, txn_data, &latest_ledger_info)
            })
            .await;
        }
    }

    async fn get_transaction_by_hash_inner(
        &self,
        accept_type: &AcceptType,
        hash: HashValue,
    ) -> BasicResultWith404<Transaction> {
        let context = self.context.clone();
        let accept_type = accept_type.clone();

        let (internal_ledger_info_opt, storage_ledger_info) =
            api_spawn_blocking(move || context.get_latest_internal_and_storage_ledger_info())
                .await?;
        let storage_version = storage_ledger_info.ledger_version.into();
        let internal_indexer_version = internal_ledger_info_opt
            .as_ref()
            .map(|info| info.ledger_version.into());
        let latest_ledger_info = internal_ledger_info_opt.unwrap_or(storage_ledger_info);

        let txn_data = self
            .get_by_hash(hash.into(), storage_version, internal_indexer_version)
            .await
            .context(format!("Failed to get transaction by hash {}", hash))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?
            .context(format!("Failed to find transaction with hash: {}", hash))
            .map_err(|_| transaction_not_found_by_hash(hash, &latest_ledger_info))?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.get_transaction_inner(&accept_type, txn_data, &latest_ledger_info)
        })
        .await
    }

    fn get_transaction_by_version_inner(
        &self,
        accept_type: &AcceptType,
        version: U64,
    ) -> BasicResultWith404<Transaction> {
        let ledger_info = self.context.get_latest_ledger_info()?;
        let txn_data = self
            .get_by_version(version.0, &ledger_info)
            .context(format!("Failed to get transaction by version {}", version))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        match txn_data {
            GetByVersionResponse::Found(txn_data) => {
                self.get_transaction_inner(accept_type, txn_data, &ledger_info)
            },
            GetByVersionResponse::VersionTooNew => {
                Err(transaction_not_found_by_version(version.0, &ledger_info))
            },
            GetByVersionResponse::VersionTooOld => Err(version_pruned(version.0, &ledger_info)),
        }
    }

    /// Converts a transaction into the outgoing type
    fn get_transaction_inner(
        &self,
        accept_type: &AcceptType,
        transaction_data: TransactionData,
        ledger_info: &LedgerInfo,
    ) -> BasicResultWith404<Transaction> {
        match accept_type {
            AcceptType::Json => {
                let state_view = self.context.latest_state_view_poem(ledger_info)?;
                let transaction = match transaction_data {
                    TransactionData::OnChain(txn) => {
                        let timestamp =
                            self.context.get_block_timestamp(ledger_info, txn.version)?;
                        state_view
                            .as_converter(
                                self.context.db.clone(),
                                self.context.indexer_reader.clone(),
                            )
                            .try_into_onchain_transaction(timestamp, txn)
                            .context("Failed to convert on chain transaction to Transaction")
                            .map_err(|err| {
                                BasicErrorWith404::internal_with_code(
                                    err,
                                    AptosErrorCode::InternalError,
                                    ledger_info,
                                )
                            })?
                    },
                    TransactionData::Pending(txn) => state_view
                        .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                        .try_into_pending_transaction(*txn)
                        .context("Failed to convert on pending transaction to Transaction")
                        .map_err(|err| {
                            BasicErrorWith404::internal_with_code(
                                err,
                                AptosErrorCode::InternalError,
                                ledger_info,
                            )
                        })?,
                };

                BasicResponse::try_from_json((transaction, ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_bcs((
                transaction_data,
                ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieves a transaction by ledger version
    fn get_by_version(
        &self,
        version: u64,
        ledger_info: &LedgerInfo,
    ) -> anyhow::Result<GetByVersionResponse> {
        if version > ledger_info.version() {
            return Ok(GetByVersionResponse::VersionTooNew);
        }
        if version < ledger_info.oldest_version() {
            return Ok(GetByVersionResponse::VersionTooOld);
        }
        Ok(GetByVersionResponse::Found(
            TransactionData::from_transaction_onchain_data(
                self.context
                    .get_transaction_by_version(version, ledger_info.version())?,
                ledger_info.version(),
            )?,
        ))
    }

    /// Retrieves a transaction by hash. First the node tries to find the transaction
    /// in the DB. If the transaction is found there, it means the transaction is
    /// committed. If it is not found there, it looks in mempool. If it is found there,
    /// it means the transaction is still pending.
    async fn get_by_hash(
        &self,
        hash: aptos_crypto::HashValue,
        storage_ledger_version: u64,
        internal_ledger_version: Option<u64>,
    ) -> anyhow::Result<Option<TransactionData>> {
        Ok(
            match self.context.get_pending_transaction_by_hash(hash).await? {
                None => {
                    let context_clone = self.context.clone();
                    tokio::task::spawn_blocking(move || {
                        context_clone.get_transaction_by_hash(hash, storage_ledger_version)
                    })
                    .await
                    .context("Failed to join task to read transaction by hash")?
                    .context("Failed to read transaction by hash from DB")?
                    .map(|t| {
                        TransactionData::from_transaction_onchain_data(
                            t,
                            internal_ledger_version.unwrap_or(storage_ledger_version),
                        )
                    })
                    .transpose()?
                },
                Some(t) => Some(t.into()),
            },
        )
    }

    /// List all transactions for an account
    fn list_by_account(
        &self,
        accept_type: &AcceptType,
        page: Page,
        address: Address,
    ) -> BasicResultWith404<Vec<Transaction>> {
        // Verify the account exists
        let account = Account::new(self.context.clone(), address, None, None, None)?;
        account.get_account_resource()?;

        let latest_ledger_info = account.latest_ledger_info;
        // TODO: Return more specific errors from within this function.
        let data = self.context.get_account_transactions(
            address.into(),
            page.start_option(),
            page.limit(&latest_ledger_info)?,
            account.ledger_version,
            &latest_ledger_info,
        )?;
        match accept_type {
            AcceptType::Json => BasicResponse::try_from_json((
                self.context
                    .render_transactions_non_sequential(&latest_ledger_info, data)?,
                &latest_ledger_info,
                BasicResponseStatus::Ok,
            )),
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((data, &latest_ledger_info, BasicResponseStatus::Ok))
            },
        }
    }

    /// Parses a single signed transaction
    fn get_signed_transaction(
        &self,
        ledger_info: &LedgerInfo,
        data: SubmitTransactionPost,
    ) -> Result<SignedTransaction, SubmitTransactionError> {
        match data {
            SubmitTransactionPost::Bcs(data) => {
                let signed_transaction: SignedTransaction =
                    bcs::from_bytes_with_limit(&data.0, MAX_RECURSIVE_TYPES_ALLOWED as usize)
                        .context("Failed to deserialize input into SignedTransaction")
                        .map_err(|err| {
                            SubmitTransactionError::bad_request_with_code(
                                err,
                                AptosErrorCode::InvalidInput,
                                ledger_info,
                            )
                        })?;
                // Verify the signed transaction
                match signed_transaction.payload() {
                    TransactionPayload::EntryFunction(entry_function) => {
                        TransactionsApi::validate_entry_function_payload_format(
                            ledger_info,
                            entry_function,
                        )?;
                    },
                    TransactionPayload::Script(script) => {
                        if script.code().is_empty() {
                            return Err(SubmitTransactionError::bad_request_with_code(
                                "Script payload bytecode must not be empty",
                                AptosErrorCode::InvalidInput,
                                ledger_info,
                            ));
                        }

                        for arg in script.ty_args() {
                            let arg = MoveType::from(arg);
                            arg.verify(0)
                                .context("Transaction script function type arg invalid")
                                .map_err(|err| {
                                    SubmitTransactionError::bad_request_with_code(
                                        err,
                                        AptosErrorCode::InvalidInput,
                                        ledger_info,
                                    )
                                })?;
                        }
                    },
                    TransactionPayload::Multisig(multisig) => {
                        if let Some(payload) = &multisig.transaction_payload {
                            match payload {
                                MultisigTransactionPayload::EntryFunction(entry_function) => {
                                    TransactionsApi::validate_entry_function_payload_format(
                                        ledger_info,
                                        entry_function,
                                    )?;
                                },
                            }
                        }
                    },

                    // Deprecated. To avoid panics when malicios users submit this
                    // payload, return an error.
                    TransactionPayload::ModuleBundle(_) => {
                        return Err(SubmitTransactionError::bad_request_with_code(
                            "Module bundle payload has been removed",
                            AptosErrorCode::InvalidInput,
                            ledger_info,
                        ))
                    },
                }
                // TODO: Verify script args?

                Ok(signed_transaction)
            },
            SubmitTransactionPost::Json(data) => self
                .context
                .latest_state_view_poem(ledger_info)?
                .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                .try_into_signed_transaction_poem(data.0, self.context.chain_id())
                .context("Failed to create SignedTransaction from SubmitTransactionRequest")
                .map_err(|err| {
                    SubmitTransactionError::bad_request_with_code(
                        err,
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    )
                }),
        }
    }

    // Validates that the module, function, and args in EntryFunction payload are correctly
    // formatted.
    fn validate_entry_function_payload_format(
        ledger_info: &LedgerInfo,
        payload: &EntryFunction,
    ) -> Result<(), SubmitTransactionError> {
        verify_module_identifier(payload.module().name().as_str())
            .context("Transaction entry function module invalid")
            .map_err(|err| {
                SubmitTransactionError::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    ledger_info,
                )
            })?;

        verify_function_identifier(payload.function().as_str())
            .context("Transaction entry function name invalid")
            .map_err(|err| {
                SubmitTransactionError::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    ledger_info,
                )
            })?;
        for arg in payload.ty_args() {
            let arg: MoveType = arg.into();
            arg.verify(0)
                .context("Transaction entry function type arg invalid")
                .map_err(|err| {
                    SubmitTransactionError::bad_request_with_code(
                        err,
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    )
                })?;
        }
        Ok(())
    }

    /// Parses a batch of signed transactions
    fn get_signed_transactions_batch(
        &self,
        ledger_info: &LedgerInfo,
        data: SubmitTransactionsBatchPost,
    ) -> Result<Vec<SignedTransaction>, SubmitTransactionError> {
        match data {
            SubmitTransactionsBatchPost::Bcs(data) => {
                let signed_transactions = bcs::from_bytes(&data.0)
                    .context("Failed to deserialize input into SignedTransaction")
                    .map_err(|err| {
                        SubmitTransactionError::bad_request_with_code(
                            err,
                            AptosErrorCode::InvalidInput,
                            ledger_info,
                        )
                    })?;
                Ok(signed_transactions)
            }
            SubmitTransactionsBatchPost::Json(data) => data
                .0
                .into_iter()
                .enumerate()
                .map(|(index, txn)| {
                    self.context.latest_state_view_poem(ledger_info)?
                        .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                        .try_into_signed_transaction_poem(txn, self.context.chain_id())
                        .context(format!("Failed to create SignedTransaction from SubmitTransactionRequest at position {}", index))
                        .map_err(|err| {
                            SubmitTransactionError::bad_request_with_code(
                                err,
                                AptosErrorCode::InvalidInput,
                                ledger_info,
                            )
                        })
                })
                .collect(),
        }
    }

    /// Submits a single transaction, and converts mempool codes to errors
    async fn create_internal(&self, txn: SignedTransaction) -> Result<(), AptosError> {
        let (mempool_status, vm_status_opt) = self
            .context
            .submit_transaction(txn)
            .await
            .context("Mempool failed to initially evaluate submitted transaction")
            .map_err(|err| {
                aptos_api_types::AptosError::new_with_error_code(err, AptosErrorCode::InternalError)
            })?;
        match mempool_status.code {
            MempoolStatusCode::Accepted => Ok(()),
            MempoolStatusCode::MempoolIsFull | MempoolStatusCode::TooManyTransactions => {
                Err(AptosError::new_with_error_code(
                    &mempool_status.message,
                    AptosErrorCode::MempoolIsFull,
                ))
            },
            MempoolStatusCode::VmError => {
                if let Some(status) = vm_status_opt {
                    Err(AptosError::new_with_vm_status(
                        format!(
                            "Invalid transaction: Type: {:?} Code: {:?}",
                            status.status_type(),
                            status
                        ),
                        AptosErrorCode::VmError,
                        status,
                    ))
                } else {
                    Err(AptosError::new_with_vm_status(
                        "Invalid transaction: unknown",
                        AptosErrorCode::VmError,
                        StatusCode::UNKNOWN_STATUS,
                    ))
                }
            },
            MempoolStatusCode::InvalidSeqNumber => Err(AptosError::new_with_error_code(
                mempool_status.message,
                AptosErrorCode::SequenceNumberTooOld,
            )),
            MempoolStatusCode::InvalidUpdate => Err(AptosError::new_with_error_code(
                mempool_status.message,
                AptosErrorCode::InvalidTransactionUpdate,
            )),
            MempoolStatusCode::UnknownStatus => Err(AptosError::new_with_error_code(
                format!("Transaction was rejected with status {}", mempool_status,),
                AptosErrorCode::InternalError,
            )),
        }
    }

    /// Submits a single transaction
    async fn create(
        &self,
        accept_type: &AcceptType,
        ledger_info: &LedgerInfo,
        txn: SignedTransaction,
    ) -> SubmitTransactionResult<PendingTransaction> {
        match self.create_internal(txn.clone()).await {
            Ok(()) => match accept_type {
                AcceptType::Json => {
                    let state_view = self
                        .context
                        .latest_state_view()
                        .context("Failed to read latest state checkpoint from DB")
                        .map_err(|e| {
                            SubmitTransactionError::internal_with_code(
                                e,
                                AptosErrorCode::InternalError,
                                ledger_info,
                            )
                        })?;

                    // We provide the pending transaction so that users have the hash associated
                    let pending_txn = state_view
                            .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                            .try_into_pending_transaction_poem(txn)
                            .context("Failed to build PendingTransaction from mempool response, even though it said the request was accepted")
                            .map_err(|err| SubmitTransactionError::internal_with_code(
                                err,
                                AptosErrorCode::InternalError,
                                ledger_info,
                            ))?;
                    SubmitTransactionResponse::try_from_json((
                        pending_txn,
                        ledger_info,
                        SubmitTransactionResponseStatus::Accepted,
                    ))
                },
                // With BCS, we don't return the pending transaction for efficiency, because there
                // is no new information.  The hash can be retrieved by hashing the original
                // transaction.
                AcceptType::Bcs => SubmitTransactionResponse::try_from_bcs((
                    (),
                    ledger_info,
                    SubmitTransactionResponseStatus::Accepted,
                )),
            },
            Err(error) => match error.error_code {
                AptosErrorCode::InternalError => Err(
                    SubmitTransactionError::internal_from_aptos_error(error, ledger_info),
                ),
                AptosErrorCode::VmError
                | AptosErrorCode::SequenceNumberTooOld
                | AptosErrorCode::InvalidTransactionUpdate => Err(
                    SubmitTransactionError::bad_request_from_aptos_error(error, ledger_info),
                ),
                AptosErrorCode::MempoolIsFull => Err(
                    SubmitTransactionError::insufficient_storage_from_aptos_error(
                        error,
                        ledger_info,
                    ),
                ),
                _ => Err(SubmitTransactionError::internal_from_aptos_error(
                    error,
                    ledger_info,
                )),
            },
        }
    }

    /// Submits a batch of transactions
    async fn create_batch(
        &self,
        accept_type: &AcceptType,
        ledger_info: &LedgerInfo,
        txns: Vec<SignedTransaction>,
    ) -> SubmitTransactionsBatchResult<TransactionsBatchSubmissionResult> {
        // Iterate through transactions keeping track of failures
        let mut txn_failures = Vec::new();
        for (idx, txn) in txns.iter().enumerate() {
            if let Err(error) = self.create_internal(txn.clone()).await {
                txn_failures.push(TransactionsBatchSingleSubmissionFailure {
                    error,
                    transaction_index: idx,
                })
            }
        }

        // Return the possible failures, and have a different success code for partial success
        let response_status = if txn_failures.is_empty() {
            SubmitTransactionsBatchResponseStatus::Accepted
        } else {
            // TODO: This should really throw an error if all fail
            SubmitTransactionsBatchResponseStatus::AcceptedPartial
        };

        SubmitTransactionsBatchResponse::try_from_rust_value((
            TransactionsBatchSubmissionResult {
                transaction_failures: txn_failures,
            },
            ledger_info,
            response_status,
            accept_type,
        ))
    }

    // TODO: This function leverages a lot of types from aptos_types, use the
    // local API types and just return those directly, instead of converting
    // from these types in render_transactions.
    /// Simulate a transaction in the VM
    ///
    /// Note: this returns a `Vec<UserTransaction>`, but for backwards compatibility, this can't
    /// be removed even though, there is only one possible transaction
    pub fn simulate(
        &self,
        accept_type: &AcceptType,
        ledger_info: LedgerInfo,
        txn: SignedTransaction,
    ) -> SimulateTransactionResult<Vec<UserTransaction>> {
        // The caller must ensure that the signature is not valid, as otherwise
        // a malicious actor could execute the transaction without their knowledge
        if txn.verify_signature().is_ok() {
            return Err(SubmitTransactionError::bad_request_with_code(
                "Simulated transactions must not have a valid signature",
                AptosErrorCode::InvalidInput,
                &ledger_info,
            ));
        }

        // Simulate transaction
        let state_view = self.context.latest_state_view_poem(&ledger_info)?;
        let (vm_status, output) =
            AptosSimulationVM::create_vm_and_simulate_signed_transaction(&txn, &state_view);
        let version = ledger_info.version();

        // Ensure that all known statuses return their values in the output (even if they aren't supposed to)
        let exe_status = ExecutionStatus::conmbine_vm_status_for_simulation(
            output.auxiliary_data(),
            output.status().clone(),
        );

        let stats_key = match txn.payload() {
            TransactionPayload::Script(_) => {
                format!("Script::{}", txn.committed_hash()).to_string()
            },
            TransactionPayload::ModuleBundle(_) => "ModuleBundle::unknown".to_string(),
            TransactionPayload::EntryFunction(entry_function) => FunctionStats::function_to_key(
                entry_function.module(),
                &entry_function.function().into(),
            ),
            TransactionPayload::Multisig(multisig) => {
                if let Some(payload) = &multisig.transaction_payload {
                    match payload {
                        MultisigTransactionPayload::EntryFunction(entry_function) => {
                            FunctionStats::function_to_key(
                                entry_function.module(),
                                &entry_function.function().into(),
                            )
                        },
                    }
                } else {
                    "Multisig::unknown".to_string()
                }
            },
        };
        self.context
            .simulate_txn_stats()
            .increment(stats_key, output.gas_used());

        // Build up a transaction from the outputs
        // All state hashes are invalid, and will be filled with 0s
        let txn = aptos_types::transaction::Transaction::UserTransaction(txn);
        let zero_hash = aptos_crypto::HashValue::zero();
        let info = aptos_types::transaction::TransactionInfo::new(
            txn.hash(),
            zero_hash,
            zero_hash,
            None,
            output.gas_used(),
            exe_status,
        );
        let mut events = output.events().to_vec();
        let _ = self
            .context
            .translate_v2_to_v1_events_for_simulation(&mut events);

        let simulated_txn = TransactionOnChainData {
            version,
            transaction: txn,
            info,
            events,
            accumulator_root_hash: zero_hash,
            changes: output.write_set().clone(),
        };

        let result = match accept_type {
            AcceptType::Json => {
                let transactions = self
                    .context
                    .render_transactions_non_sequential(&ledger_info, vec![simulated_txn])?;

                // Users can only make requests to simulate UserTransactions, so unpack
                // the Vec<Transaction> into Vec<UserTransaction>.
                let mut user_transactions = Vec::new();
                for transaction in transactions.into_iter() {
                    match transaction {
                        Transaction::UserTransaction(mut user_txn) => {
                            match &vm_status {
                                VMStatus::Error {
                                    message: Some(msg), ..
                                }
                                | VMStatus::ExecutionFailure {
                                    message: Some(msg), ..
                                } => {
                                    user_txn.info.vm_status +=
                                        format!("\nExecution failed with message: {}", msg)
                                            .as_str();
                                },
                                _ => (),
                            }
                            user_transactions.push(user_txn);
                        },
                        _ => {
                            return Err(SubmitTransactionError::internal_with_code(
                                "Simulation transaction resulted in a non-UserTransaction",
                                AptosErrorCode::InternalError,
                                &ledger_info,
                            ))
                        },
                    }
                }
                BasicResponse::try_from_json((
                    user_transactions,
                    &ledger_info,
                    BasicResponseStatus::Ok,
                ))
            },
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((simulated_txn, &ledger_info, BasicResponseStatus::Ok))
            },
        };

        result.map(|r| r.with_gas_used(Some(output.gas_used())))
    }

    /// Encode message as BCS
    pub fn get_signing_message(
        &self,
        accept_type: &AcceptType,
        request: EncodeSubmissionRequest,
    ) -> BasicResult<HexEncodedBytes> {
        // We don't want to encourage people to use this API if they can sign the request directly
        if accept_type == &AcceptType::Bcs {
            return Err(BasicError::bad_request_with_code_no_info(
                "BCS is not supported for encode submission",
                AptosErrorCode::BcsNotSupported,
            ));
        }

        let ledger_info = self.context.get_latest_ledger_info()?;
        let state_view = self.context.latest_state_view_poem(&ledger_info)?;
        let raw_txn: RawTransaction = state_view
            .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
            .try_into_raw_transaction_poem(request.transaction, self.context.chain_id())
            .context("The given transaction is invalid")
            .map_err(|err| {
                BasicError::bad_request_with_code(err, AptosErrorCode::InvalidInput, &ledger_info)
            })?;

        let raw_message = match request.secondary_signers {
            Some(secondary_signer_addresses) => signing_message(
                &RawTransactionWithData::new_multi_agent(
                    raw_txn,
                    secondary_signer_addresses
                        .into_iter()
                        .map(|v| v.into())
                        .collect(),
                ),
            )
            .context("Invalid transaction to generate signing message")
            .map_err(|err| {
                BasicError::bad_request_with_code(err, AptosErrorCode::InvalidInput, &ledger_info)
            })?,
            None => raw_txn
                .signing_message()
                .context("Invalid transaction to generate signing message")
                .map_err(|err| {
                    BasicError::bad_request_with_code(
                        err,
                        AptosErrorCode::InvalidInput,
                        &ledger_info,
                    )
                })?,
        };

        BasicResponse::try_from_json((
            HexEncodedBytes::from(raw_message),
            &ledger_info,
            BasicResponseStatus::Ok,
        ))
    }
}

fn override_gas_parameters(
    signed_txn: &SignedTransaction,
    max_gas_amount: Option<u64>,
    gas_unit_price: Option<u64>,
) -> SignedTransaction {
    let payload = signed_txn.payload();

    let raw_txn = RawTransaction::new(
        signed_txn.sender(),
        signed_txn.sequence_number(),
        payload.clone(),
        max_gas_amount.unwrap_or_else(|| signed_txn.max_gas_amount()),
        gas_unit_price.unwrap_or_else(|| signed_txn.gas_unit_price()),
        signed_txn.expiration_timestamp_secs(),
        signed_txn.chain_id(),
    );

    // TODO: Check that signature is null, this would just be helpful for downstream use
    SignedTransaction::new_signed_transaction(raw_txn, signed_txn.authenticator())
}

enum GetByVersionResponse {
    VersionTooNew,
    VersionTooOld,
    Found(TransactionData),
}
