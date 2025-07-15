// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    requests::{
        DataRequest::{
            GetEpochEndingLedgerInfos, GetNewTransactionDataWithProof,
            GetNewTransactionOutputsWithProof, GetNewTransactionsOrOutputsWithProof,
            GetNewTransactionsWithProof, GetNumberOfStatesAtVersion, GetServerProtocolVersion,
            GetStateValuesWithProof, GetStorageServerSummary, GetTransactionDataWithProof,
            GetTransactionOutputsWithProof, GetTransactionsOrOutputsWithProof,
            GetTransactionsWithProof, SubscribeTransactionDataWithProof,
            SubscribeTransactionOutputsWithProof, SubscribeTransactionsOrOutputsWithProof,
            SubscribeTransactionsWithProof,
        },
        TransactionDataRequestType,
    },
    responses::Error::DegenerateRangeError,
    Epoch, StorageServiceRequest, COMPRESSION_SUFFIX_LABEL,
};
use aptos_compression::{client::CompressionClient, CompressedData};
use aptos_config::config::{
    AptosDataClientConfig, StorageServiceConfig, MAX_APPLICATION_MESSAGE_SIZE,
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use num_traits::{PrimInt, Zero};
#[cfg(test)]
use proptest::prelude::{any, Arbitrary, BoxedStrategy, Strategy};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Display, Formatter},
};
use thiserror::Error;

// Useful file constants
pub const NUM_MICROSECONDS_IN_SECOND: u64 = 1_000_000;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Data range cannot be degenerate!")]
    DegenerateRangeError,
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
    #[error("Unexpected response error: {0}")]
    UnexpectedResponseError(String),
}

impl From<aptos_compression::Error> for Error {
    fn from(error: aptos_compression::Error) -> Self {
        Error::UnexpectedErrorEncountered(error.to_string())
    }
}

/// A storage service response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum StorageServiceResponse {
    CompressedResponse(String, CompressedData), // Store the label and the data (e.g., for logging/metrics)
    RawResponse(DataResponse),
}

impl StorageServiceResponse {
    /// Creates a new response and performs compression if required
    pub fn new(data_response: DataResponse, perform_compression: bool) -> Result<Self, Error> {
        if perform_compression {
            // Serialize and compress the raw data
            let raw_data = bcs::to_bytes(&data_response)
                .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
            let compressed_data = aptos_compression::compress(
                raw_data,
                CompressionClient::StateSync,
                MAX_APPLICATION_MESSAGE_SIZE,
            )?;

            // Create the compressed response
            let label = data_response.get_label().to_string() + COMPRESSION_SUFFIX_LABEL;
            Ok(StorageServiceResponse::CompressedResponse(
                label,
                compressed_data,
            ))
        } else {
            Ok(StorageServiceResponse::RawResponse(data_response))
        }
    }

    /// Returns the data response regardless of the inner format
    pub fn get_data_response(&self) -> Result<DataResponse, Error> {
        match self {
            StorageServiceResponse::CompressedResponse(_, compressed_data) => {
                let raw_data = aptos_compression::decompress(
                    compressed_data,
                    CompressionClient::StateSync,
                    MAX_APPLICATION_MESSAGE_SIZE,
                )?;
                let data_response = bcs::from_bytes::<DataResponse>(&raw_data)
                    .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?;
                Ok(data_response)
            },
            StorageServiceResponse::RawResponse(data_response) => Ok(data_response.clone()),
        }
    }

    /// Returns a summary label for the response
    pub fn get_label(&self) -> String {
        match self {
            StorageServiceResponse::CompressedResponse(label, _) => label.clone(),
            StorageServiceResponse::RawResponse(data_response) => {
                data_response.get_label().to_string()
            },
        }
    }

    /// Returns true iff the data response is compressed
    pub fn is_compressed(&self) -> bool {
        matches!(self, Self::CompressedResponse(_, _))
    }
}

/// A useful type to hold optional transaction data
pub type TransactionOrOutputListWithProof = (
    Option<TransactionListWithProof>,
    Option<TransactionOutputListWithProof>,
);

/// A single data response.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum DataResponse {
    EpochEndingLedgerInfos(EpochChangeProof),
    NewTransactionOutputsWithProof((TransactionOutputListWithProof, LedgerInfoWithSignatures)),
    NewTransactionsWithProof((TransactionListWithProof, LedgerInfoWithSignatures)),
    NumberOfStatesAtVersion(u64),
    ServerProtocolVersion(ServerProtocolVersion),
    StateValueChunkWithProof(StateValueChunkWithProof),
    StorageServerSummary(StorageServerSummary),
    TransactionOutputsWithProof(TransactionOutputListWithProof),
    TransactionsWithProof(TransactionListWithProof),
    NewTransactionsOrOutputsWithProof((TransactionOrOutputListWithProof, LedgerInfoWithSignatures)),
    TransactionsOrOutputsWithProof(TransactionOrOutputListWithProof),
}

impl DataResponse {
    /// Returns a summary label for the response
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::EpochEndingLedgerInfos(_) => "epoch_ending_ledger_infos",
            Self::NewTransactionOutputsWithProof(_) => "new_transaction_outputs_with_proof",
            Self::NewTransactionsWithProof(_) => "new_transactions_with_proof",
            Self::NumberOfStatesAtVersion(_) => "number_of_states_at_version",
            Self::ServerProtocolVersion(_) => "server_protocol_version",
            Self::StateValueChunkWithProof(_) => "state_value_chunk_with_proof",
            Self::StorageServerSummary(_) => "storage_server_summary",
            Self::TransactionOutputsWithProof(_) => "transaction_outputs_with_proof",
            Self::TransactionsWithProof(_) => "transactions_with_proof",
            Self::NewTransactionsOrOutputsWithProof(_) => "new_transactions_or_outputs_with_proof",
            Self::TransactionsOrOutputsWithProof(_) => "transactions_or_outputs_with_proof",
        }
    }
}

impl Display for DataResponse {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        // To prevent log spamming, we only display storage response data for summaries
        let data = match self {
            DataResponse::StorageServerSummary(storage_summary) => {
                format!("{:?}", storage_summary)
            },
            _ => "...".into(),
        };
        write!(
            f,
            "Storage service response: {}, data: {}",
            self.get_label(),
            data
        )
    }
}

impl TryFrom<StorageServiceResponse> for StateValueChunkWithProof {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::StateValueChunkWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected state_value_chunk_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for EpochChangeProof {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::EpochEndingLedgerInfos(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected epoch_ending_ledger_infos, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse>
    for (TransactionOutputListWithProof, LedgerInfoWithSignatures)
{
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::NewTransactionOutputsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected new_transaction_outputs_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for (TransactionListWithProof, LedgerInfoWithSignatures) {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::NewTransactionsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected new_transactions_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for u64 {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::NumberOfStatesAtVersion(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected number_of_states_at_version, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for ServerProtocolVersion {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::ServerProtocolVersion(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected server_protocol_version, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for StorageServerSummary {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::StorageServerSummary(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected storage_server_summary, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for TransactionOutputListWithProof {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::TransactionOutputsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected transaction_outputs_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for TransactionListWithProof {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::TransactionsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected transactions_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse>
    for (TransactionOrOutputListWithProof, LedgerInfoWithSignatures)
{
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::NewTransactionsOrOutputsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected new_transactions_or_outputs_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

impl TryFrom<StorageServiceResponse> for TransactionOrOutputListWithProof {
    type Error = crate::responses::Error;

    fn try_from(response: StorageServiceResponse) -> crate::Result<Self, Self::Error> {
        let data_response = response.get_data_response()?;
        match data_response {
            DataResponse::TransactionsOrOutputsWithProof(inner) => Ok(inner),
            _ => Err(Error::UnexpectedResponseError(format!(
                "expected transactions_or_outputs_with_proof, found {}",
                data_response.get_label()
            ))),
        }
    }
}

/// The protocol version run by this server. Clients request this first to
/// identify what API calls and data requests the server supports.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServerProtocolVersion {
    pub protocol_version: u64, // The storage server version run by this instance.
}

/// A storage server summary, containing a summary of the information held
/// by the corresponding server instance. This is useful for identifying the
/// data that a server instance can provide, as well as relevant metadata.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct StorageServerSummary {
    pub protocol_metadata: ProtocolMetadata,
    pub data_summary: DataSummary,
}

// TODO: it probably makes sense to move this logic to the data client,
// instead of having it attached to the storage server summary.
impl StorageServerSummary {
    pub fn can_service(
        &self,
        aptos_data_client_config: &AptosDataClientConfig,
        time_service: TimeService,
        request: &StorageServiceRequest,
    ) -> bool {
        self.protocol_metadata.can_service(request)
            && self
                .data_summary
                .can_service(aptos_data_client_config, time_service, request)
    }
}

/// A summary of the protocol metadata for the storage service instance, such as
/// the maximum chunk sizes supported for different requests.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProtocolMetadata {
    pub max_epoch_chunk_size: u64, // The max number of epochs the server can return in a single chunk
    pub max_state_chunk_size: u64, // The max number of states the server can return in a single chunk
    pub max_transaction_chunk_size: u64, // The max number of transactions the server can return in a single chunk
    pub max_transaction_output_chunk_size: u64, // The max number of transaction outputs the server can return in a single chunk
}

impl ProtocolMetadata {
    /// We deem all requests serviceable, even if the requested chunk
    /// sizes are larger than the maximum sizes that can be served (the
    /// response will simply be truncated on the server side).
    pub fn can_service(&self, _request: &StorageServiceRequest) -> bool {
        true // TODO: figure out if should eventually remove this
    }
}

impl Default for ProtocolMetadata {
    fn default() -> Self {
        let config = StorageServiceConfig::default();
        Self {
            max_epoch_chunk_size: config.max_epoch_chunk_size,
            max_transaction_chunk_size: config.max_transaction_chunk_size,
            max_transaction_output_chunk_size: config.max_transaction_output_chunk_size,
            max_state_chunk_size: config.max_state_chunk_size,
        }
    }
}

/// A summary of the data actually held by the storage service instance.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataSummary {
    /// The ledger info corresponding to the highest synced version in storage.
    /// This indicates the highest version and epoch that storage can prove.
    pub synced_ledger_info: Option<LedgerInfoWithSignatures>,
    /// The range of epoch ending ledger infos in storage, e.g., if the range
    /// is [(X,Y)], it means all epoch ending ledger infos for epochs X->Y
    /// (inclusive) are held.
    pub epoch_ending_ledger_infos: Option<CompleteDataRange<Epoch>>,
    /// The range of states held in storage, e.g., if the range is
    /// [(X,Y)], it means all states are held for every version X->Y
    /// (inclusive).
    pub states: Option<CompleteDataRange<Version>>,
    /// The range of transactions held in storage, e.g., if the range is
    /// [(X,Y)], it means all transactions for versions X->Y (inclusive) are held.
    pub transactions: Option<CompleteDataRange<Version>>,
    /// The range of transaction outputs held in storage, e.g., if the range
    /// is [(X,Y)], it means all transaction outputs for versions X->Y
    /// (inclusive) are held.
    pub transaction_outputs: Option<CompleteDataRange<Version>>,
}

impl DataSummary {
    /// Returns true iff the request can be serviced
    pub fn can_service(
        &self,
        aptos_data_client_config: &AptosDataClientConfig,
        time_service: TimeService,
        request: &StorageServiceRequest,
    ) -> bool {
        match &request.data_request {
            GetServerProtocolVersion | GetStorageServerSummary => true,
            GetEpochEndingLedgerInfos(request) => {
                let desired_range =
                    match CompleteDataRange::new(request.start_epoch, request.expected_end_epoch) {
                        Ok(desired_range) => desired_range,
                        Err(_) => return false,
                    };
                self.epoch_ending_ledger_infos
                    .map(|range| range.superset_of(&desired_range))
                    .unwrap_or(false)
            },
            GetNewTransactionOutputsWithProof(_) => can_service_optimistic_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            GetNewTransactionsWithProof(_) => can_service_optimistic_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            GetNewTransactionsOrOutputsWithProof(_) => can_service_optimistic_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            GetNumberOfStatesAtVersion(version) => self
                .states
                .map(|range| range.contains(*version))
                .unwrap_or(false),
            GetStateValuesWithProof(request) => {
                let proof_version = request.version;

                let can_serve_states = self
                    .states
                    .map(|range| range.contains(request.version))
                    .unwrap_or(false);

                let can_create_proof = self
                    .synced_ledger_info
                    .as_ref()
                    .map(|li| li.ledger_info().version() >= proof_version)
                    .unwrap_or(false);

                can_serve_states && can_create_proof
            },
            GetTransactionOutputsWithProof(request) => self
                .can_service_transaction_outputs_with_proof(
                    request.start_version,
                    request.end_version,
                    request.proof_version,
                ),
            GetTransactionsWithProof(request) => self.can_service_transactions_with_proof(
                request.start_version,
                request.end_version,
                request.proof_version,
            ),
            GetTransactionsOrOutputsWithProof(request) => self
                .can_service_transactions_or_outputs_with_proof(
                    request.start_version,
                    request.end_version,
                    request.proof_version,
                ),
            SubscribeTransactionOutputsWithProof(_) => can_service_subscription_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            SubscribeTransactionsOrOutputsWithProof(_) => can_service_subscription_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            SubscribeTransactionsWithProof(_) => can_service_subscription_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),

            // Transaction data v2 requests (transactions with auxiliary data)
            GetTransactionDataWithProof(request) => match request.transaction_data_request_type {
                TransactionDataRequestType::TransactionData(_) => self
                    .can_service_transactions_with_proof(
                        request.start_version,
                        request.end_version,
                        request.proof_version,
                    ),
                TransactionDataRequestType::TransactionOutputData => self
                    .can_service_transaction_outputs_with_proof(
                        request.start_version,
                        request.end_version,
                        request.proof_version,
                    ),
                TransactionDataRequestType::TransactionOrOutputData(_) => self
                    .can_service_transactions_or_outputs_with_proof(
                        request.start_version,
                        request.end_version,
                        request.proof_version,
                    ),
            },
            GetNewTransactionDataWithProof(_) => can_service_optimistic_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
            SubscribeTransactionDataWithProof(_) => can_service_subscription_request(
                aptos_data_client_config,
                time_service,
                self.synced_ledger_info.as_ref(),
            ),
        }
    }

    /// Returns true iff the peer can create a proof for the given version
    fn can_create_proof(&self, proof_version: u64) -> bool {
        self.synced_ledger_info
            .as_ref()
            .map(|li| li.ledger_info().version() >= proof_version)
            .unwrap_or(false)
    }

    /// Returns true iff the peer can service the transaction outputs in the given range
    fn can_service_transaction_outputs(&self, desired_range: &CompleteDataRange<u64>) -> bool {
        self.transaction_outputs
            .map(|range| range.superset_of(desired_range))
            .unwrap_or(false)
    }

    /// Returns true iff the peer can service the transactions in the given range
    fn can_service_transactions(&self, desired_range: &CompleteDataRange<u64>) -> bool {
        self.transactions
            .map(|range| range.superset_of(desired_range))
            .unwrap_or(false)
    }

    /// Returns true iff the peer can service the transaction outputs and proof
    fn can_service_transaction_outputs_with_proof(
        &self,
        start_version: u64,
        end_version: u64,
        proof_version: u64,
    ) -> bool {
        let desired_range = match CompleteDataRange::new(start_version, end_version) {
            Ok(desired_range) => desired_range,
            Err(_) => return false,
        };

        let can_service_outputs = self.can_service_transaction_outputs(&desired_range);
        let can_create_proof = self.can_create_proof(proof_version);
        can_service_outputs && can_create_proof
    }

    /// Returns true iff the peer can service the transactions or outputs and proof
    fn can_service_transactions_or_outputs_with_proof(
        &self,
        start_version: u64,
        end_version: u64,
        proof_version: u64,
    ) -> bool {
        let desired_range = match CompleteDataRange::new(start_version, end_version) {
            Ok(desired_range) => desired_range,
            Err(_) => return false,
        };

        let can_service_transactions = self.can_service_transactions(&desired_range);
        let can_service_outputs = self.can_service_transaction_outputs(&desired_range);
        let can_create_proof = self.can_create_proof(proof_version);
        can_service_transactions && can_service_outputs && can_create_proof
    }

    /// Returns true iff the peer can service the transactions and proof
    fn can_service_transactions_with_proof(
        &self,
        start_version: u64,
        end_version: u64,
        proof_version: u64,
    ) -> bool {
        let desired_range = match CompleteDataRange::new(start_version, end_version) {
            Ok(desired_range) => desired_range,
            Err(_) => return false,
        };

        let can_service_transactions = self.can_service_transactions(&desired_range);
        let can_create_proof = self.can_create_proof(proof_version);
        can_service_transactions && can_create_proof
    }

    /// Returns the version of the synced ledger info (if one exists)
    pub fn get_synced_ledger_info_version(&self) -> Option<u64> {
        self.synced_ledger_info
            .as_ref()
            .map(|ledger_info| ledger_info.ledger_info().version())
    }
}

/// Returns true iff an optimistic data request can be serviced
/// by the peer with the given synced ledger info.
fn can_service_optimistic_request(
    aptos_data_client_config: &AptosDataClientConfig,
    time_service: TimeService,
    synced_ledger_info: Option<&LedgerInfoWithSignatures>,
) -> bool {
    let max_lag_secs = aptos_data_client_config.max_optimistic_fetch_lag_secs;
    check_synced_ledger_lag(synced_ledger_info, time_service, max_lag_secs)
}

/// Returns true iff a subscription data request can be serviced
/// by the peer with the given synced ledger info.
fn can_service_subscription_request(
    aptos_data_client_config: &AptosDataClientConfig,
    time_service: TimeService,
    synced_ledger_info: Option<&LedgerInfoWithSignatures>,
) -> bool {
    let max_lag_secs = aptos_data_client_config.max_subscription_lag_secs;
    check_synced_ledger_lag(synced_ledger_info, time_service, max_lag_secs)
}

/// Returns true iff the synced ledger info timestamp
/// is within the given lag (in seconds).
fn check_synced_ledger_lag(
    synced_ledger_info: Option<&LedgerInfoWithSignatures>,
    time_service: TimeService,
    max_lag_secs: u64,
) -> bool {
    if let Some(synced_ledger_info) = synced_ledger_info {
        // Get the ledger info timestamp (in microseconds)
        let ledger_info_timestamp_usecs = synced_ledger_info.ledger_info().timestamp_usecs();

        // Get the current timestamp and max version lag (in microseconds)
        let current_timestamp_usecs = time_service.now_unix_time().as_micros() as u64;
        let max_version_lag_usecs = max_lag_secs * NUM_MICROSECONDS_IN_SECOND;

        // Return true iff the synced ledger info timestamp is within the max version lag
        ledger_info_timestamp_usecs + max_version_lag_usecs > current_timestamp_usecs
    } else {
        false // No synced ledger info was found!
    }
}

/// A struct representing a contiguous, non-empty data range (lowest to highest,
/// inclusive) where data is complete (i.e. there are no missing pieces of data).
///
/// This is used to provide a summary of the data currently held in storage, e.g.
/// a `CompleteDataRange<Version>` of (A,B) means all versions A->B (inclusive).
///
/// Note: `CompleteDataRanges` are never degenerate (lowest > highest) and the
/// range length is always expressible without overflowing. Constructing a
/// degenerate range via `new` will return an `Err`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CompleteDataRange<T> {
    lowest: T,
    highest: T,
}

fn range_length_checked<T: PrimInt>(lowest: T, highest: T) -> crate::Result<T, Error> {
    // len = highest - lowest + 1
    // Note: the order of operations here is important; we need to subtract first
    // before we (+1) to ensure we don't underflow when highest == lowest.
    highest
        .checked_sub(&lowest)
        .and_then(|value| value.checked_add(&T::one()))
        .ok_or(DegenerateRangeError)
}

impl<T: PrimInt> CompleteDataRange<T> {
    pub fn new(lowest: T, highest: T) -> crate::Result<Self, Error> {
        if lowest > highest || range_length_checked(lowest, highest).is_err() {
            Err(DegenerateRangeError)
        } else {
            Ok(Self { lowest, highest })
        }
    }

    /// Create a data range given the lower bound and the length of the range.
    pub fn from_len(lowest: T, len: T) -> crate::Result<Self, Error> {
        // highest = lowest + len - 1
        // Note: the order of operations here is important
        let highest = len
            .checked_sub(&T::one())
            .and_then(|addend| lowest.checked_add(&addend))
            .ok_or(DegenerateRangeError)?;
        Self::new(lowest, highest)
    }

    #[inline]
    pub fn lowest(&self) -> T {
        self.lowest
    }

    #[inline]
    pub fn highest(&self) -> T {
        self.highest
    }

    /// Returns the length of the data range.
    #[inline]
    pub fn len(&self) -> crate::Result<T, Error> {
        self.highest
            .checked_sub(&self.lowest)
            .and_then(|value| value.checked_add(&T::one()))
            .ok_or(DegenerateRangeError)
    }

    /// Returns true iff the given item is within this range
    pub fn contains(&self, item: T) -> bool {
        self.lowest <= item && item <= self.highest
    }

    /// Returns true iff this range is a superset of the other data range.
    pub fn superset_of(&self, other: &Self) -> bool {
        self.lowest <= other.lowest && other.highest <= self.highest
    }
}

impl<T: Zero> CompleteDataRange<T> {
    pub fn from_genesis(highest: T) -> Self {
        Self {
            lowest: T::zero(),
            highest,
        }
    }
}

impl<'de, T> serde::Deserialize<'de> for CompleteDataRange<T>
where
    T: PrimInt + serde::Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> crate::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        #[derive(Deserialize)]
        #[serde(rename = "CompleteDataRange")]
        struct Value<U> {
            lowest: U,
            highest: U,
        }

        let value = Value::<T>::deserialize(deserializer)?;
        Self::new(value.lowest, value.highest).map_err(D::Error::custom)
    }
}

#[cfg(test)]
impl<T> Arbitrary for CompleteDataRange<T>
where
    T: PrimInt + Arbitrary + 'static,
{
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        (any::<T>(), any::<T>())
            .prop_filter_map("degenerate range", |(lowest, highest)| {
                CompleteDataRange::new(lowest, highest).ok()
            })
            .boxed()
    }
}
