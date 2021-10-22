// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{DataClientResponse, DiemDataClient, Error, ResponseError};
use async_trait::async_trait;
use diem_config::network_id::PeerNetworkId;
use network::{application::interface::NetworkInterface, protocols::rpc::error::RpcError};
use rand::seq::SliceRandom;
use std::{
    convert::TryInto,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use storage_service_client::StorageServiceClient;
use storage_service_types::{
    AccountStatesChunkWithProofRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};

// TODO(philiphayes): does this belong in a different crate? I feel like we're
// accumulating a lot of tiny crates though...

// TODO(philiphayes): configuration / pass as argument?
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(10_000);

/// A [`DiemDataClient`] that fulfills requests from remote peers' Storage Service
/// over DiemNet.
///
/// The `DiemNetDataClient`:
///
/// 1. Sends requests to connected DiemNet peers.
/// 2. Does basic type conversions and error handling on the responses.
/// 3. Routes requests to peers that advertise availability for that data.
/// 4. Maintains peer scores based on each peer's observed quality of service
///    and upper client reports of invalid or malicious data.
/// 5. Selects high quality peers to send each request to.
/// 6. Exposes a condensed data summary of our peers' data advertisements.
///
/// The client currently assumes 1-request => 1-response. Streaming responses
/// are handled at an upper layer.
///
/// The client is expected to be cloneable and usable from many concurrent tasks
/// and/or threads.
#[derive(Clone, Debug)]
pub struct DiemNetDataClient {
    network: StorageServiceClient,
    next_response_id: Arc<AtomicU64>,
}

impl DiemNetDataClient {
    pub fn new(network: StorageServiceClient) -> Self {
        Self {
            network,
            next_response_id: Arc::new(AtomicU64::new(0)),
        }
    }

    fn sample_peer(&self) -> Option<PeerNetworkId> {
        // very dumb. just get this working e2e
        let peer_infos = self.network.peer_metadata_storage();
        let all_connected = peer_infos
            .networks()
            .flat_map(|network_id| {
                peer_infos
                    .read_filtered(network_id, |(_, peer_info)| peer_info.is_connected())
                    .into_keys()
            })
            .collect::<Vec<_>>();
        all_connected.choose(&mut rand::thread_rng()).copied()
    }

    // TODO(philiphayes): this should be generic in DiemDataClient
    pub async fn send_request(
        &self,
        // TODO(philiphayes): should be a separate DataClient type
        request: StorageServiceRequest,
    ) -> Result<DataClientResponse, Error> {
        let peer = self
            .sample_peer()
            .ok_or_else(|| Error::DataIsUnavailable("no connected diemnet peers".to_owned()))?;

        let result = self
            .network
            .send_request(peer, request, DEFAULT_TIMEOUT)
            .await;

        match result {
            Ok(response) => Ok(DataClientResponse {
                response_id: self.next_response_id.fetch_add(1, Ordering::Relaxed),
                response_payload: response.try_into()?,
            }),
            Err(storage_service_client::Error::RpcError(err)) => match err {
                RpcError::NotConnected(_) => Err(Error::DataIsUnavailable(err.to_string())),
                RpcError::TimedOut => Err(Error::TimeoutWaitingForResponse(err.to_string())),
                _ => Err(Error::UnexpectedErrorEncountered(err.to_string())),
            },
            Err(storage_service_client::Error::StorageServiceError(err)) => {
                Err(Error::UnexpectedErrorEncountered(err.to_string()))
            }
        }

        // TODO(philiphayes): update peer scores on error
    }
}

/// (start..=end).len()
fn range_len(start: u64, end: u64) -> Result<u64, Error> {
    // len = end - start + 1
    let len = end.checked_sub(start).ok_or_else(|| {
        Error::InvalidRequest(format!("end ({}) must be >= start ({})", end, start))
    })?;
    let len = len
        .checked_add(1)
        .ok_or_else(|| Error::InvalidRequest(format!("end ({}) must not be u64::MAX", end)))?;
    Ok(len)
}

#[async_trait]
impl DiemDataClient for DiemNetDataClient {
    async fn get_account_states_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> Result<DataClientResponse, Error> {
        self.send_request(StorageServiceRequest::GetAccountStatesChunkWithProof(
            AccountStatesChunkWithProofRequest {
                version,
                start_account_index: start_index,
                expected_num_account_states: range_len(start_index, end_index)?,
            },
        ))
        .await
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
        expected_end_epoch: u64,
    ) -> Result<DataClientResponse, Error> {
        self.send_request(StorageServiceRequest::GetEpochEndingLedgerInfos(
            EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            },
        ))
        .await
    }

    fn get_global_data_summary(&self) -> Result<DataClientResponse, Error> {
        todo!()
    }

    async fn get_number_of_account_states(
        &self,
        version: u64,
    ) -> Result<DataClientResponse, Error> {
        self.send_request(StorageServiceRequest::GetNumberOfAccountsAtVersion(version))
            .await
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
    ) -> Result<DataClientResponse, Error> {
        self.send_request(StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                expected_num_outputs: range_len(start_version, end_version)?,
            },
        ))
        .await
    }

    async fn get_transactions_with_proof(
        &self,
        proof_version: u64,
        start_version: u64,
        end_version: u64,
        include_events: bool,
    ) -> Result<DataClientResponse, Error> {
        self.send_request(StorageServiceRequest::GetTransactionsWithProof(
            TransactionsWithProofRequest {
                proof_version,
                start_version,
                expected_num_transactions: range_len(start_version, end_version)?,
                include_events,
            },
        ))
        .await
    }

    async fn notify_bad_response(
        &self,
        _response_id: u64,
        _response_error: ResponseError,
    ) -> Result<(), Error> {
        todo!()
    }
}
