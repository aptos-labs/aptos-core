// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    accounts::Account,
    context::{api_spawn_blocking, Context, FunctionStats},
    failpoint::fail_point_poem,
    generate_error_response, generate_success_response, metrics,
    page::Page,
    response::{
        api_disabled, api_forbidden, transaction_not_found_by_hash,
        transaction_not_found_by_version, version_pruned, BadRequestError, BasicError,
        BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResult, BasicResultWith404,
        ForbiddenError, InsufficientStorageError, InternalError,
    },
    response_axum::{AptosErrorResponse, AptosResponse},
    view_function::convert_view_function_error,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    transaction::{PersistedAuxiliaryInfo, TransactionSummary},
    verify_function_identifier, verify_module_identifier, Address, AptosError, AptosErrorCode,
    AsConverter, EncodeSubmissionRequest, GasEstimation, GasEstimationBcs, HashValue,
    HexEncodedBytes, LedgerInfo, MoveType, PendingTransaction, SubmitTransactionRequest,
    Transaction, TransactionData, TransactionOnChainData, TransactionsBatchSingleSubmissionFailure,
    TransactionsBatchSubmissionResult, UserTransaction, VerifyInput, VerifyInputWithRecursion, U64,
};
use aptos_crypto::signing_message;
use aptos_logger::error;
use aptos_types::{
    account_address::AccountAddress,
    mempool_status::MempoolStatusCode,
    transaction::{
        EntryFunction, ExecutionStatus, MultisigTransactionPayload, RawTransaction,
        RawTransactionWithData, Script, SignedTransaction, TransactionExecutable,
        TransactionPayload, TransactionPayloadInner,
    },
    vm_status::StatusCode,
    AptosCoinType, CoinType,
};
use aptos_vm::{AptosSimulationVM, AptosVM};
use move_core_types::{ident_str, language_storage::ModuleId, vm_status::VMStatus};
use std::{cmp::min, sync::Arc, time::Duration};

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

#[derive(Debug)]
pub enum SubmitTransactionPost {
    Json(SubmitTransactionRequest),
    Bcs(Vec<u8>),
}

impl VerifyInput for SubmitTransactionPost {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            SubmitTransactionPost::Json(inner) => inner.verify(),
            SubmitTransactionPost::Bcs(_) => Ok(()),
        }
    }
}

#[derive(Debug)]
pub enum SubmitTransactionsBatchPost {
    Json(Vec<SubmitTransactionRequest>),
    Bcs(Vec<u8>),
}

impl VerifyInput for SubmitTransactionsBatchPost {
    fn verify(&self) -> anyhow::Result<()> {
        match self {
            SubmitTransactionsBatchPost::Json(inner) => {
                for request in inner.iter() {
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

impl TransactionsApi {
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
}

impl TransactionsApi {
    const MAX_SIGNED_TRANSACTION_DEPTH: usize = 16;

    /// List all transactions paging by ledger version
    pub(crate) fn list(
        &self,
        accept_type: &AcceptType,
        page: Page,
    ) -> BasicResultWith404<Vec<Transaction>> {
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

    pub(crate) async fn wait_transaction_by_hash_inner(
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

    pub(crate) async fn get_transaction_by_hash_inner(
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

    pub(crate) fn get_transaction_by_version_inner(
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
    pub(crate) fn get_by_version(
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

    /// List sequence number based transactions for an account
    pub(crate) fn list_ordered_txns_by_account(
        &self,
        accept_type: &AcceptType,
        page: Page,
        address: Address,
    ) -> BasicResultWith404<Vec<Transaction>> {
        // Verify the account exists
        let account = Account::new(self.context.clone(), address, None, None, None)?;

        let latest_ledger_info = account.latest_ledger_info;
        // TODO: Return more specific errors from within this function.
        let data = self.context.get_account_ordered_transactions(
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

    /// List transaction summaries of committed transactions of an account
    pub(crate) fn list_txn_summaries_by_account(
        &self,
        accept_type: &AcceptType,
        address: Address,
        start_version: Option<U64>,
        end_version: Option<U64>,
        limit: u16,
    ) -> BasicResultWith404<Vec<TransactionSummary>> {
        let (latest_ledger_info, ledger_version) = self
            .context
            .get_latest_ledger_info_and_verify_lookup_version(None)?;

        // TODO: Return more specific errors from within this function.
        match self.context.get_account_transaction_summaries(
            address.into(),
            start_version.map(|v| v.into()),
            end_version.map(|v| v.into()),
            limit,
            ledger_version,
            &latest_ledger_info,
        ) {
            Ok(data) => match accept_type {
                AcceptType::Json => BasicResponse::try_from_json((
                    self.context
                        .render_transaction_summaries(&latest_ledger_info, data)?,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                )),
                AcceptType::Bcs => BasicResponse::try_from_bcs((
                    data,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                )),
            },
            Err(e) => {
                error!("list_all_txn_summaries_by_account error: {:?}", e);
                Err(e)
            },
        }
    }

    fn validate_script(
        ledger_info: &LedgerInfo,
        script: &Script,
    ) -> Result<(), SubmitTransactionError> {
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
        Ok(())
    }

    /// Parses a single signed transaction
    pub(crate) fn get_signed_transaction(
        &self,
        ledger_info: &LedgerInfo,
        data: SubmitTransactionPost,
    ) -> Result<SignedTransaction, SubmitTransactionError> {
        match data {
            SubmitTransactionPost::Bcs(data) => {
                let signed_transaction: SignedTransaction =
                    bcs::from_bytes_with_limit(&data, Self::MAX_SIGNED_TRANSACTION_DEPTH)
                        .context("Failed to deserialize input into SignedTransaction")
                        .map_err(|err| {
                            SubmitTransactionError::bad_request_with_code(
                                err,
                                AptosErrorCode::InvalidInput,
                                ledger_info,
                            )
                        })?;
                // Verify the signed transaction
                self.validate_signed_transaction_payload(ledger_info, &signed_transaction)?;

                Ok(signed_transaction)
            },
            SubmitTransactionPost::Json(data) => self
                .context
                .latest_state_view_poem(ledger_info)?
                .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                .try_into_signed_transaction_poem(data, self.context.chain_id())
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

    /// Validates a signed transaction's payload
    fn validate_signed_transaction_payload(
        &self,
        ledger_info: &LedgerInfo,
        signed_transaction: &SignedTransaction,
    ) -> Result<(), SubmitTransactionError> {
        match signed_transaction.payload() {
            TransactionPayload::EntryFunction(entry_function) => {
                TransactionsApi::validate_entry_function_payload_format(
                    ledger_info,
                    entry_function,
                )?;
            },
            TransactionPayload::Script(script) => {
                TransactionsApi::validate_script(ledger_info, script)?;
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
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            }) => match executable {
                TransactionExecutable::Script(script) => {
                    TransactionsApi::validate_script(ledger_info, script)?;
                    if extra_config.is_multisig() {
                        return Err(SubmitTransactionError::bad_request_with_code(
                            "Script transaction payload must not be a multisig transaction",
                            AptosErrorCode::InvalidInput,
                            ledger_info,
                        ));
                    }
                },
                TransactionExecutable::EntryFunction(entry_function) => {
                    TransactionsApi::validate_entry_function_payload_format(
                        ledger_info,
                        entry_function,
                    )?;
                },
                TransactionExecutable::Empty => {
                    if !extra_config.is_multisig() {
                        return Err(SubmitTransactionError::bad_request_with_code(
                            "Empty transaction payload must be a multisig transaction",
                            AptosErrorCode::InvalidInput,
                            ledger_info,
                        ));
                    }
                },
                TransactionExecutable::Encrypted => {
                    return Err(SubmitTransactionError::bad_request_with_code(
                        "Encrypted executable is not supported in PayloadV1",
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    ));
                },
            },
            TransactionPayload::EncryptedPayload(payload) => {
                if !self.context.node_config.api.allow_encrypted_txns_submission {
                    return Err(SubmitTransactionError::bad_request_with_code(
                        "Encrypted Transaction submission is not allowed yet",
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    ));
                }

                if !payload.is_encrypted() {
                    return Err(SubmitTransactionError::bad_request_with_code(
                        "Encrypted transaction must be in encrypted state",
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    ));
                }

                if let Err(e) = payload.verify(signed_transaction.sender()) {
                    return Err(SubmitTransactionError::bad_request_with_code(
                        e.context("Encrypted transaction payload could not be verified"),
                        AptosErrorCode::InvalidInput,
                        ledger_info,
                    ));
                }
            },
        }
        Ok(())
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
    pub(crate) fn get_signed_transactions_batch(
        &self,
        ledger_info: &LedgerInfo,
        data: SubmitTransactionsBatchPost,
    ) -> Result<Vec<SignedTransaction>, SubmitTransactionError> {
        match data {
            SubmitTransactionsBatchPost::Bcs(data) => {
                let signed_transactions: Vec<SignedTransaction> =
                    bcs::from_bytes_with_limit(&data, Self::MAX_SIGNED_TRANSACTION_DEPTH)
                        .context("Failed to deserialize input into SignedTransaction")
                        .map_err(|err| {
                            SubmitTransactionError::bad_request_with_code(
                                err,
                                AptosErrorCode::InvalidInput,
                                ledger_info,
                            )
                        })?;
                // Verify each signed transaction
                for signed_transaction in signed_transactions.iter() {
                    self.validate_signed_transaction_payload(ledger_info, signed_transaction)?;
                }
                Ok(signed_transactions)
            }
            SubmitTransactionsBatchPost::Json(data) => data
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
            MempoolStatusCode::RejectedByFilter => Err(AptosError::new_with_error_code(
                mempool_status.message,
                AptosErrorCode::RejectedByFilter,
            )),
        }
    }

    /// Submits a single transaction
    pub(crate) async fn create(
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
    pub(crate) async fn create_batch(
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
        } else if txn_failures.len() == txns.len() {
            return Err(SubmitTransactionError::bad_request_with_code(
                "All transactions submitted were invalid.",
                AptosErrorCode::InvalidInput,
                ledger_info,
            ));
        } else {
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

        if txn
            .raw_transaction_ref()
            .payload_ref()
            .is_encrypted_variant()
        {
            return Err(SubmitTransactionError::bad_request_with_code(
                "Encrypted transactions cannot be simulated",
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
            TransactionPayload::Payload(TransactionPayloadInner::V1 {
                executable,
                extra_config,
            }) => {
                let mut stats_key: String = "V2::".to_string();
                if extra_config.is_multisig() {
                    stats_key += "Multisig::";
                };
                if extra_config.is_orderless() {
                    stats_key += "Orderless::";
                }
                if let TransactionExecutable::Script(_) = executable {
                    stats_key += format!("Script::{}", txn.committed_hash()).as_str();
                } else if let TransactionExecutable::EntryFunction(entry_function) = executable {
                    stats_key += FunctionStats::function_to_key(
                        entry_function.module(),
                        &entry_function.function().into(),
                    )
                    .as_str();
                } else if let TransactionExecutable::Empty = executable {
                    stats_key += "unknown";
                };
                stats_key
            },
            TransactionPayload::EncryptedPayload(_) => {
                unreachable!("Encrypted transactions must not be simulated")
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
            txn.committed_hash(),
            zero_hash,
            zero_hash,
            None,
            output.gas_used(),
            exe_status,
            None,
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

    pub(crate) fn list_auxiliary_infos(
        &self,
        accept_type: &AcceptType,
        page: Page,
    ) -> BasicResultWith404<Vec<PersistedAuxiliaryInfo>> {
        let latest_ledger_info = self.context.get_latest_ledger_info()?;
        let ledger_version = latest_ledger_info.ledger_version;

        let limit = page.limit(&latest_ledger_info)?;
        let start_version = page.compute_start(limit, ledger_version.0, &latest_ledger_info)?;

        // Use iterator for more efficient batch retrieval
        let iterator = self
            .context
            .db
            .get_persisted_auxiliary_info_iterator(start_version, limit as usize)
            .context("Failed to get auxiliary info iterator from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?;

        let mut raw_auxiliary_infos = Vec::new();
        for result in iterator {
            let raw_aux_info = result
                .context("Failed to read auxiliary info from iterator")
                .map_err(|err| {
                    BasicErrorWith404::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        &latest_ledger_info,
                    )
                })?;
            raw_auxiliary_infos.push(raw_aux_info);
        }

        match accept_type {
            AcceptType::Json => {
                // Transform to API types for JSON (user-friendly, extensible)
                let api_auxiliary_infos: Vec<PersistedAuxiliaryInfo> = raw_auxiliary_infos
                    .into_iter()
                    .map(PersistedAuxiliaryInfo::from)
                    .collect();
                BasicResponse::try_from_json((
                    api_auxiliary_infos,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            },
            AcceptType::Bcs => {
                // Use raw core types for BCS (backward compatible, versioned enum)
                BasicResponse::try_from_bcs((
                    raw_auxiliary_infos,
                    &latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            },
        }
    }
}

/// Inner implementation of estimate_gas_price that returns Axum-native types.
/// Used by Axum handlers to avoid the poem_to_axum_response bridge.
pub(crate) fn estimate_gas_price_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
) -> Result<AptosResponse<GasEstimation>, AptosErrorResponse> {
    let ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let gas_estimation = context.estimate_gas_price::<AptosErrorResponse>(&ledger_info)?;
    TransactionsApi::log_gas_estimation(&gas_estimation);

    match accept_type {
        AcceptType::Json => AptosResponse::try_from_json(gas_estimation, &ledger_info),
        AcceptType::Bcs => {
            let gas_estimation_bcs = GasEstimationBcs {
                gas_estimate: gas_estimation.gas_estimate,
            };
            AptosResponse::try_from_bcs(gas_estimation_bcs, &ledger_info)
        },
    }
}

/// Framework-agnostic inner for listing transactions. Returns Axum-native types.
pub fn list_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    page: Page,
) -> Result<AptosResponse<Vec<Transaction>>, AptosErrorResponse> {
    let latest_ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let ledger_version = latest_ledger_info.version();

    let limit = page.limit::<AptosErrorResponse>(&latest_ledger_info)?;
    let start_version =
        page.compute_start::<AptosErrorResponse>(limit, ledger_version, &latest_ledger_info)?;
    let data = context
        .get_transactions(start_version, limit, ledger_version)
        .context("Failed to read raw transactions from storage")
        .map_err(|err| {
            AptosErrorResponse::internal_with_code(
                err,
                AptosErrorCode::InternalError,
                &latest_ledger_info,
            )
        })?;

    match accept_type {
        AcceptType::Json => {
            let timestamp = context
                .get_block_timestamp::<AptosErrorResponse>(&latest_ledger_info, start_version)?;
            AptosResponse::try_from_json(
                context.render_transactions_sequential::<AptosErrorResponse>(
                    &latest_ledger_info,
                    data,
                    timestamp,
                )?,
                &latest_ledger_info,
            )
        },
        AcceptType::Bcs => AptosResponse::try_from_bcs(data, &latest_ledger_info),
    }
}

/// Framework-agnostic inner for get transaction by version. Returns Axum-native types.
pub fn get_transaction_by_version_inner_axum(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    version: U64,
) -> Result<AptosResponse<Transaction>, AptosErrorResponse> {
    let ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let api = TransactionsApi {
        context: context.clone(),
    };
    let txn_data = api
        .get_by_version(version.0, &ledger_info)
        .context(format!("Failed to get transaction by version {}", version))
        .map_err(|err| {
            AptosErrorResponse::internal_with_code(err, AptosErrorCode::InternalError, &ledger_info)
        })?;

    match txn_data {
        GetByVersionResponse::Found(txn_data) => match accept_type {
            AcceptType::Bcs => AptosResponse::try_from_bcs(txn_data, &ledger_info),
            AcceptType::Json => {
                let state_view =
                    context.latest_state_view_poem::<AptosErrorResponse>(&ledger_info)?;
                let transaction = match txn_data {
                    TransactionData::OnChain(txn) => {
                        let timestamp = context
                            .get_block_timestamp::<AptosErrorResponse>(&ledger_info, txn.version)?;
                        state_view
                            .as_converter(context.db.clone(), context.indexer_reader.clone())
                            .try_into_onchain_transaction(timestamp, txn)
                            .context("Failed to convert on chain transaction to Transaction")
                            .map_err(|err| {
                                AptosErrorResponse::internal_with_code(
                                    err,
                                    AptosErrorCode::InternalError,
                                    &ledger_info,
                                )
                            })?
                    },
                    TransactionData::Pending(txn) => state_view
                        .as_converter(context.db.clone(), context.indexer_reader.clone())
                        .try_into_pending_transaction(*txn)
                        .context("Failed to convert on pending transaction to Transaction")
                        .map_err(|err| {
                            AptosErrorResponse::internal_with_code(
                                err,
                                AptosErrorCode::InternalError,
                                &ledger_info,
                            )
                        })?,
                };
                AptosResponse::try_from_json(transaction, &ledger_info)
            },
        },
        GetByVersionResponse::VersionTooNew => Err(
            crate::response_axum::transaction_not_found_by_version(version.0, &ledger_info),
        ),
        GetByVersionResponse::VersionTooOld => Err(crate::response_axum::version_pruned(
            version.0,
            &ledger_info,
        )),
    }
}

/// Framework-agnostic inner for listing auxiliary infos. Returns Axum-native types.
pub fn list_auxiliary_infos_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    page: Page,
) -> Result<AptosResponse<Vec<PersistedAuxiliaryInfo>>, AptosErrorResponse> {
    let latest_ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let ledger_version = latest_ledger_info.ledger_version;

    let limit = page.limit::<AptosErrorResponse>(&latest_ledger_info)?;
    let start_version =
        page.compute_start::<AptosErrorResponse>(limit, ledger_version.0, &latest_ledger_info)?;

    let iterator = context
        .db
        .get_persisted_auxiliary_info_iterator(start_version, limit as usize)
        .context("Failed to get auxiliary info iterator from storage")
        .map_err(|err| {
            AptosErrorResponse::internal_with_code(
                err,
                AptosErrorCode::InternalError,
                &latest_ledger_info,
            )
        })?;

    let mut raw_auxiliary_infos = Vec::new();
    for result in iterator {
        let raw_aux_info = result
            .context("Failed to read auxiliary info from iterator")
            .map_err(|err| {
                AptosErrorResponse::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?;
        raw_auxiliary_infos.push(raw_aux_info);
    }

    match accept_type {
        AcceptType::Bcs => AptosResponse::try_from_bcs(raw_auxiliary_infos, &latest_ledger_info),
        AcceptType::Json => {
            let api_auxiliary_infos: Vec<PersistedAuxiliaryInfo> = raw_auxiliary_infos
                .iter()
                .map(|r| PersistedAuxiliaryInfo::from(*r))
                .collect();
            AptosResponse::try_from_json(api_auxiliary_infos, &latest_ledger_info)
        },
    }
}

/// Framework-agnostic inner for listing ordered transactions by account. Returns Axum-native types.
pub fn list_ordered_txns_by_account_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    page: Page,
    address: Address,
) -> Result<AptosResponse<Vec<Transaction>>, AptosErrorResponse> {
    let account = Account::new(context.clone(), address, None, None, None)?;
    let latest_ledger_info = account.latest_ledger_info;

    let data = context.get_account_ordered_transactions::<AptosErrorResponse>(
        address.into(),
        page.start_option(),
        page.limit::<AptosErrorResponse>(&latest_ledger_info)?,
        account.ledger_version,
        &latest_ledger_info,
    )?;

    match accept_type {
        AcceptType::Json => AptosResponse::try_from_json(
            context.render_transactions_non_sequential::<AptosErrorResponse>(
                &latest_ledger_info,
                data,
            )?,
            &latest_ledger_info,
        ),
        AcceptType::Bcs => AptosResponse::try_from_bcs(data, &latest_ledger_info),
    }
}

/// Framework-agnostic inner for listing transaction summaries by account. Returns Axum-native types.
pub fn list_txn_summaries_by_account_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    address: Address,
    start_version: Option<U64>,
    end_version: Option<U64>,
    limit: u16,
) -> Result<AptosResponse<Vec<TransactionSummary>>, AptosErrorResponse> {
    let (latest_ledger_info, ledger_version) =
        context.get_latest_ledger_info_and_verify_lookup_version::<AptosErrorResponse>(None)?;

    match context.get_account_transaction_summaries::<AptosErrorResponse>(
        address.into(),
        start_version.map(|v| v.into()),
        end_version.map(|v| v.into()),
        limit,
        ledger_version,
        &latest_ledger_info,
    ) {
        Ok(data) => match accept_type {
            AcceptType::Json => AptosResponse::try_from_json(
                context.render_transaction_summaries::<AptosErrorResponse>(
                    &latest_ledger_info,
                    data,
                )?,
                &latest_ledger_info,
            ),
            AcceptType::Bcs => AptosResponse::try_from_bcs(data, &latest_ledger_info),
        },
        Err(e) => {
            error!("list_all_txn_summaries_by_account error: {:?}", e);
            Err(e)
        },
    }
}

/// Framework-agnostic inner for get signing message. Returns Axum-native types.
pub fn get_signing_message_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    request: EncodeSubmissionRequest,
) -> Result<AptosResponse<HexEncodedBytes>, AptosErrorResponse> {
    if accept_type == &AcceptType::Bcs {
        return Err(AptosErrorResponse::bad_request(
            "BCS is not supported for encode submission",
            AptosErrorCode::BcsNotSupported,
            None,
        ));
    }

    let ledger_info = context.get_latest_ledger_info::<AptosErrorResponse>()?;
    let state_view = context.latest_state_view_poem::<AptosErrorResponse>(&ledger_info)?;
    let raw_txn: RawTransaction = state_view
        .as_converter(context.db.clone(), context.indexer_reader.clone())
        .try_into_raw_transaction_poem(request.transaction, context.chain_id())
        .context("The given transaction is invalid")
        .map_err(|err| {
            AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, Some(&ledger_info))
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
            AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, Some(&ledger_info))
        })?,
        None => raw_txn
            .signing_message()
            .context("Invalid transaction to generate signing message")
            .map_err(|err| {
                AptosErrorResponse::bad_request(
                    err,
                    AptosErrorCode::InvalidInput,
                    Some(&ledger_info),
                )
            })?,
    };

    AptosResponse::try_from_json(HexEncodedBytes::from(raw_message), &ledger_info)
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

pub(crate) enum GetByVersionResponse {
    VersionTooNew,
    VersionTooOld,
    Found(TransactionData),
}

pub async fn create_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    ledger_info: &LedgerInfo,
    txn: SignedTransaction,
) -> Result<AptosResponse<PendingTransaction>, AptosErrorResponse> {
    let txn_api = TransactionsApi {
        context: context.clone(),
    };
    match txn_api.create_internal(txn.clone()).await {
        Ok(()) => match accept_type {
            AcceptType::Json => {
                let state_view = context
                    .latest_state_view()
                    .context("Failed to read latest state checkpoint from DB")
                    .map_err(|e| {
                        AptosErrorResponse::internal(
                            e,
                            AptosErrorCode::InternalError,
                            Some(ledger_info),
                        )
                    })?;
                let pending_txn = state_view
                    .as_converter(context.db.clone(), context.indexer_reader.clone())
                    .try_into_pending_transaction_poem(txn)
                    .context("Failed to build PendingTransaction from mempool response")
                    .map_err(|err| {
                        AptosErrorResponse::internal(
                            err,
                            AptosErrorCode::InternalError,
                            Some(ledger_info),
                        )
                    })?;
                AptosResponse::try_from_json_with_status((
                    pending_txn,
                    ledger_info,
                    axum::http::StatusCode::ACCEPTED,
                ))
            },
            AcceptType::Bcs => AptosResponse::try_from_bcs_with_status((
                (),
                ledger_info,
                axum::http::StatusCode::ACCEPTED,
            )),
        },
        Err(error) => Err(AptosErrorResponse::new(
            match error.error_code {
                AptosErrorCode::InternalError => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                AptosErrorCode::MempoolIsFull => axum::http::StatusCode::INSUFFICIENT_STORAGE,
                _ => axum::http::StatusCode::BAD_REQUEST,
            },
            error,
            Some(ledger_info),
        )),
    }
}

/// Framework-agnostic inner for get transaction by hash. Returns Axum-native types.
pub async fn get_transaction_by_hash_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    hash: HashValue,
) -> Result<AptosResponse<Transaction>, AptosErrorResponse> {
    let txn_api = TransactionsApi {
        context: context.clone(),
    };
    let (internal_ledger_info_opt, storage_ledger_info) = api_spawn_blocking({
        let context_clone = context.clone();
        move || context_clone.get_latest_internal_and_storage_ledger_info::<AptosErrorResponse>()
    })
    .await?;
    let storage_version = storage_ledger_info.ledger_version.into();
    let internal_indexer_version = internal_ledger_info_opt
        .as_ref()
        .map(|info| info.ledger_version.into());
    let latest_ledger_info = internal_ledger_info_opt.unwrap_or(storage_ledger_info);

    let txn_data = txn_api
        .get_by_hash(hash.into(), storage_version, internal_indexer_version)
        .await
        .context(format!("Failed to get transaction by hash {}", hash))
        .map_err(|err| {
            AptosErrorResponse::internal_with_code(
                err,
                AptosErrorCode::InternalError,
                &latest_ledger_info,
            )
        })?
        .context(format!("Failed to find transaction with hash: {}", hash))
        .map_err(|_| {
            crate::response_axum::transaction_not_found_by_hash(hash, &latest_ledger_info)
        })?;

    let context_for_convert = context.clone();
    let accept_type_for_convert = accept_type.clone();
    api_spawn_blocking(move || {
        get_transaction_inner_axum(
            &context_for_convert,
            &accept_type_for_convert,
            txn_data,
            &latest_ledger_info,
        )
    })
    .await
}

fn get_transaction_inner_axum(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    transaction_data: TransactionData,
    ledger_info: &LedgerInfo,
) -> Result<AptosResponse<Transaction>, AptosErrorResponse> {
    match accept_type {
        AcceptType::Json => {
            let state_view = context.latest_state_view_poem::<AptosErrorResponse>(ledger_info)?;
            let transaction = match transaction_data {
                TransactionData::OnChain(txn) => {
                    let timestamp = context
                        .get_block_timestamp::<AptosErrorResponse>(ledger_info, txn.version)?;
                    state_view
                        .as_converter(context.db.clone(), context.indexer_reader.clone())
                        .try_into_onchain_transaction(timestamp, txn)
                        .context("Failed to convert on chain transaction to Transaction")
                        .map_err(|err| {
                            AptosErrorResponse::internal_with_code(
                                err,
                                AptosErrorCode::InternalError,
                                ledger_info,
                            )
                        })?
                },
                TransactionData::Pending(txn) => state_view
                    .as_converter(context.db.clone(), context.indexer_reader.clone())
                    .try_into_pending_transaction(*txn)
                    .context("Failed to convert on pending transaction to Transaction")
                    .map_err(|err| {
                        AptosErrorResponse::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            ledger_info,
                        )
                    })?,
            };
            AptosResponse::try_from_json(transaction, ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_bcs(transaction_data, ledger_info),
    }
}

/// Framework-agnostic inner for wait transaction by hash. Returns Axum-native types.
pub async fn wait_transaction_by_hash_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    hash: HashValue,
    timeout_ms: u64,
    poll_interval_ms: u64,
) -> Result<AptosResponse<Transaction>, AptosErrorResponse> {
    let txn_api = TransactionsApi {
        context: context.clone(),
    };
    let start_time = std::time::Instant::now();
    loop {
        let (internal_ledger_info_opt, storage_ledger_info) = api_spawn_blocking({
            let context_clone = context.clone();
            move || {
                context_clone.get_latest_internal_and_storage_ledger_info::<AptosErrorResponse>()
            }
        })
        .await?;
        let storage_version = storage_ledger_info.ledger_version.into();
        let internal_ledger_version = internal_ledger_info_opt
            .as_ref()
            .map(|info| info.ledger_version.into());
        let latest_ledger_info = internal_ledger_info_opt.unwrap_or(storage_ledger_info);
        let txn_data = txn_api
            .get_by_hash(hash.into(), storage_version, internal_ledger_version)
            .await
            .context(format!("Failed to get transaction by hash {}", hash))
            .map_err(|err| {
                AptosErrorResponse::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?
            .context(format!("Failed to find transaction with hash: {}", hash))
            .map_err(|_| {
                crate::response_axum::transaction_not_found_by_hash(hash, &latest_ledger_info)
            })?;

        if matches!(txn_data, TransactionData::Pending(_))
            && (start_time.elapsed().as_millis() as u64) < timeout_ms
        {
            tokio::time::sleep(Duration::from_millis(poll_interval_ms)).await;
            continue;
        }

        let context_for_convert = context.clone();
        let accept_type_for_convert = accept_type.clone();
        return api_spawn_blocking(move || {
            get_transaction_inner_axum(
                &context_for_convert,
                &accept_type_for_convert,
                txn_data,
                &latest_ledger_info,
            )
        })
        .await;
    }
}

/// Framework-agnostic inner for create batch. Returns Axum-native types.
pub async fn create_batch_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    ledger_info: &LedgerInfo,
    txns: Vec<SignedTransaction>,
) -> Result<AptosResponse<TransactionsBatchSubmissionResult>, AptosErrorResponse> {
    let txn_api = TransactionsApi {
        context: context.clone(),
    };
    let mut txn_failures = Vec::new();
    for (idx, txn) in txns.iter().enumerate() {
        if let Err(error) = txn_api.create_internal(txn.clone()).await {
            txn_failures.push(TransactionsBatchSingleSubmissionFailure {
                error,
                transaction_index: idx,
            })
        }
    }

    if txn_failures.is_empty() {
        let result = TransactionsBatchSubmissionResult {
            transaction_failures: txn_failures,
        };
        match accept_type {
            AcceptType::Json => AptosResponse::try_from_json_with_status((
                result,
                ledger_info,
                axum::http::StatusCode::ACCEPTED,
            )),
            AcceptType::Bcs => AptosResponse::try_from_bcs_with_status((
                result,
                ledger_info,
                axum::http::StatusCode::ACCEPTED,
            )),
        }
    } else if txn_failures.len() == txns.len() {
        Err(AptosErrorResponse::bad_request(
            "All transactions submitted were invalid.",
            AptosErrorCode::InvalidInput,
            Some(ledger_info),
        ))
    } else {
        let result = TransactionsBatchSubmissionResult {
            transaction_failures: txn_failures,
        };
        AptosResponse::try_from_rust_value_with_status((
            result,
            ledger_info,
            axum::http::StatusCode::PARTIAL_CONTENT,
            accept_type,
        ))
    }
}

/// Framework-agnostic inner for simulate. Returns Axum-native types. NOT async.
pub fn simulate_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    ledger_info: LedgerInfo,
    txn: SignedTransaction,
) -> Result<AptosResponse<Vec<UserTransaction>>, AptosErrorResponse> {
    simulate_inner_impl(context, accept_type, &ledger_info, txn)
}

fn simulate_inner_impl(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    ledger_info: &LedgerInfo,
    txn: SignedTransaction,
) -> Result<AptosResponse<Vec<UserTransaction>>, AptosErrorResponse> {
    if txn.verify_signature().is_ok() {
        return Err(AptosErrorResponse::bad_request(
            "Simulated transactions must not have a valid signature",
            AptosErrorCode::InvalidInput,
            Some(ledger_info),
        ));
    }

    if txn
        .raw_transaction_ref()
        .payload_ref()
        .is_encrypted_variant()
    {
        return Err(AptosErrorResponse::bad_request(
            "Encrypted transactions cannot be simulated",
            AptosErrorCode::InvalidInput,
            Some(ledger_info),
        ));
    }

    let state_view = context.latest_state_view_poem::<AptosErrorResponse>(ledger_info)?;
    let (vm_status, output) =
        AptosSimulationVM::create_vm_and_simulate_signed_transaction(&txn, &state_view);
    let version = ledger_info.version();

    let exe_status = ExecutionStatus::conmbine_vm_status_for_simulation(
        output.auxiliary_data(),
        output.status().clone(),
    );

    let stats_key = match txn.payload() {
        TransactionPayload::Script(_) => format!("Script::{}", txn.committed_hash()),
        TransactionPayload::ModuleBundle(_) => "ModuleBundle::unknown".to_string(),
        TransactionPayload::EntryFunction(entry_function) => FunctionStats::function_to_key(
            entry_function.module(),
            &entry_function.function().into(),
        ),
        TransactionPayload::Multisig(multisig) => {
            if let Some(MultisigTransactionPayload::EntryFunction(entry_function)) =
                &multisig.transaction_payload
            {
                FunctionStats::function_to_key(
                    entry_function.module(),
                    &entry_function.function().into(),
                )
            } else {
                "Multisig::unknown".to_string()
            }
        },
        TransactionPayload::Payload(TransactionPayloadInner::V1 {
            executable,
            extra_config,
        }) => {
            let mut stats_key: String = "V2::".to_string();
            if extra_config.is_multisig() {
                stats_key += "Multisig::";
            }
            if extra_config.is_orderless() {
                stats_key += "Orderless::";
            }
            match executable {
                TransactionExecutable::Script(_) => {
                    stats_key += format!("Script::{}", txn.committed_hash()).as_str();
                },
                TransactionExecutable::EntryFunction(entry_function) => {
                    stats_key += FunctionStats::function_to_key(
                        entry_function.module(),
                        &entry_function.function().into(),
                    )
                    .as_str();
                },
                TransactionExecutable::Empty => stats_key += "unknown",
                TransactionExecutable::Encrypted => stats_key += "unknown",
            };
            stats_key
        },
        TransactionPayload::EncryptedPayload(_) => {
            unreachable!("Encrypted transactions must not be simulated")
        },
    };
    context
        .simulate_txn_stats()
        .increment(stats_key, output.gas_used());

    let txn_inner = aptos_types::transaction::Transaction::UserTransaction(txn);
    let zero_hash = aptos_crypto::HashValue::zero();
    let info = aptos_types::transaction::TransactionInfo::new(
        txn_inner.committed_hash(),
        zero_hash,
        zero_hash,
        None,
        output.gas_used(),
        exe_status,
        None,
    );
    let mut events = output.events().to_vec();
    let _ = context.translate_v2_to_v1_events_for_simulation(&mut events);

    let simulated_txn = TransactionOnChainData {
        version,
        transaction: txn_inner,
        info,
        events,
        accumulator_root_hash: zero_hash,
        changes: output.write_set().clone(),
    };

    let response = match accept_type {
        AcceptType::Json => {
            let transactions = context
                .render_transactions_non_sequential::<AptosErrorResponse>(ledger_info, vec![
                    simulated_txn,
                ])?;
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
                                    format!("\nExecution failed with message: {}", msg).as_str();
                            },
                            _ => (),
                        }
                        user_transactions.push(user_txn);
                    },
                    _ => {
                        return Err(AptosErrorResponse::internal_with_code(
                            "Simulation transaction resulted in a non-UserTransaction",
                            AptosErrorCode::InternalError,
                            ledger_info,
                        ));
                    },
                }
            }
            AptosResponse::try_from_json(user_transactions, ledger_info)?
        },
        AcceptType::Bcs => {
            let bytes = bcs::to_bytes(&simulated_txn).map_err(|e| {
                AptosErrorResponse::internal(e, AptosErrorCode::InternalError, Some(ledger_info))
            })?;
            AptosResponse::<Vec<UserTransaction>>::new_bcs(
                axum::http::StatusCode::OK,
                bytes,
                ledger_info,
            )
        },
    };

    Ok(response.with_gas_used(Some(output.gas_used())))
}
