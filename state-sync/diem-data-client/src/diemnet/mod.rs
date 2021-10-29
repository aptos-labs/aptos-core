// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AdvertisedData, DataClientPayload, DataClientResponse, DiemDataClient, Error,
    GlobalDataSummary, OptimalChunkSizes, ResponseError,
};
use async_trait::async_trait;
use diem_config::network_id::PeerNetworkId;
use diem_id_generator::{IdGenerator, U64IdGenerator};
use diem_infallible::RwLock;
use diem_logger::trace;
use diem_time_service::{TimeService, TimeServiceTrait};
use futures::StreamExt;
use network::{
    application::interface::NetworkInterface,
    protocols::{rpc::error::RpcError, wire::handshake::v1::ProtocolId},
};
use rand::seq::SliceRandom;
use std::{collections::HashMap, convert::TryInto, sync::Arc, time::Duration};
use storage_service_client::StorageServiceClient;
use storage_service_types::{
    AccountStatesChunkWithProofRequest, EpochEndingLedgerInfoRequest, StorageServerSummary,
    StorageServiceRequest, StorageServiceResponse, TransactionOutputsWithProofRequest,
    TransactionsWithProofRequest,
};

#[cfg(test)]
mod tests;

// TODO(philiphayes): does this belong in a different crate? I feel like we're
// accumulating a lot of tiny crates though...

// TODO(philiphayes): configuration / pass as argument?
const DEFAULT_TIMEOUT: Duration = Duration::from_millis(10_000);
pub const DATA_SUMMARY_POLL_INTERVAL: Duration = Duration::from_millis(100);

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
    /// The underlying DiemNet storage service client.
    network_client: StorageServiceClient,
    /// All of the data-client specific data we have on each network peer.
    peer_states: Arc<RwLock<PeerStates>>,
    /// A cached, aggregate data summary of all unbanned peers' data summaries.
    global_summary_cache: Arc<RwLock<GlobalDataSummary>>,
    /// Used for generating the next request/response id.
    response_id_generator: Arc<U64IdGenerator>,
}

impl DiemNetDataClient {
    pub fn new(
        time_service: TimeService,
        network_client: StorageServiceClient,
    ) -> (Self, DataSummaryPoller) {
        let client = Self {
            network_client,
            peer_states: Arc::new(RwLock::new(PeerStates::new())),
            global_summary_cache: Arc::new(RwLock::new(GlobalDataSummary::empty())),
            response_id_generator: Arc::new(U64IdGenerator::new()),
        };
        let poller =
            DataSummaryPoller::new(time_service, client.clone(), DATA_SUMMARY_POLL_INTERVAL);
        (client, poller)
    }

    fn next_response_id(&self) -> u64 {
        self.response_id_generator.next()
    }

    /// Update a peer's data summary.
    fn update_summary(&self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.peer_states.write().update_summary(peer, summary)
    }

    /// Recompute and update the global data summary cache.
    fn update_global_summary_cache(&self) {
        let aggregate = self.peer_states.read().aggregate_summary();
        *self.global_summary_cache.write() = aggregate;
    }

    /// Choose a connected peer that can service the given request. Returns an
    /// error if no such peer can be found.
    fn choose_peer(&self, request: &StorageServiceRequest) -> Result<PeerNetworkId, Error> {
        let all_connected = {
            let network_peer_metadata = self.network_client.peer_metadata_storage();
            network_peer_metadata
                .networks()
                .flat_map(|network_id| {
                    network_peer_metadata
                        .read_filtered(network_id, |(_, peer_metadata)| {
                            peer_metadata.is_connected()
                                && peer_metadata.supports_protocol(ProtocolId::StorageServiceRpc)
                        })
                        .into_keys()
                })
                .collect::<Vec<_>>()
        };

        if all_connected.is_empty() {
            return Err(Error::DataIsUnavailable(
                "no connected diemnet peers".to_owned(),
            ));
        }

        let internal_peer_states = self.peer_states.read();
        let all_serviceable = all_connected
            .into_iter()
            .filter(|peer| internal_peer_states.can_service_request(peer, request))
            .collect::<Vec<_>>();

        all_serviceable
            .choose(&mut rand::thread_rng())
            .copied()
            .ok_or_else(|| {
                Error::DataIsUnavailable(
                    "no connected peers are advertising that they can serve this data range"
                        .to_owned(),
                )
            })
    }

    // TODO(philiphayes): this should be generic in DiemDataClient trait
    pub async fn send_request(
        &self,
        // TODO(philiphayes): should be a separate DataClient type?
        request: StorageServiceRequest,
    ) -> Result<DataClientResponse, Error> {
        let peer = self.choose_peer(&request)?;
        self.send_request_to_peer(peer, request).await
    }

    async fn send_request_to_peer(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
    ) -> Result<DataClientResponse, Error> {
        let response_id = self.next_response_id();
        let response = self.send_request_to_peer_inner(peer, request).await;
        Ok(DataClientResponse {
            response_id,
            response_payload: response?.try_into()?,
        })
    }

    async fn send_request_to_peer_inner(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let result = self
            .network_client
            .send_request(peer, request, DEFAULT_TIMEOUT)
            .await;
        match result {
            Ok(response) => Ok(response),
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

    async fn send_data_summary_request(
        &self,
        peer: PeerNetworkId,
    ) -> Result<StorageServerSummary, Error> {
        let response = self
            .send_request_to_peer_inner(peer, StorageServiceRequest::GetStorageServerSummary)
            .await?;
        match response {
            StorageServiceResponse::StorageServerSummary(summary) => Ok(summary),
            _ => Err(Error::InvalidResponse("unexpected response".to_owned())),
        }
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
    fn get_global_data_summary(&self) -> Result<DataClientResponse, Error> {
        // TODO(philiphayes): feels awkward to create a DataClientResponse here
        // rather than just return the type directly. what does the response id
        // mean here?
        Ok(DataClientResponse {
            response_id: self.next_response_id(),
            response_payload: DataClientPayload::GlobalDataSummary(
                self.global_summary_cache.read().clone(),
            ),
        })
    }

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
        if proof_version < end_version {
            return Err(Error::InvalidRequest(format!(
                "proof_version ({}) must be >= end_version ({})",
                proof_version, end_version
            )));
        }
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

// TODO(philiphayes):
// + ownership b/w poller and data client a bit murky
// + how to stop poller loop? ideally all data client refs get dropped and it
//   just works.
// + would need to make data client contain weak refs somehow when in poller...
// + or maybe poller needs to not depend on data client?
// + an explicit close method seems unsafe / easy to forget...
// + ofc, in prod we will never cancel lol
pub struct DataSummaryPoller {
    time_service: TimeService,
    data_client: DiemNetDataClient,
    poll_interval: Duration,
}

impl DataSummaryPoller {
    fn new(
        time_service: TimeService,
        data_client: DiemNetDataClient,
        poll_interval: Duration,
    ) -> Self {
        Self {
            time_service,
            data_client,
            poll_interval,
        }
    }

    pub async fn start(self) {
        trace!("DataSummaryPoller: start");

        let ticker = self.time_service.interval(self.poll_interval);
        futures::pin_mut!(ticker);

        // TODO(philiphayes): rather than polling one at a time, maybe do
        // round-robin with a few concurrent polls.
        loop {
            // wait for next round to poll
            ticker.next().await;

            trace!("DataSummaryPoller: polling next peer");

            // just sample a random peer for now. do something smarter here in
            // the future.
            let maybe_peer = self
                .data_client
                .choose_peer(&StorageServiceRequest::GetStorageServerSummary);

            trace!("DataSummaryPoller: maybe polling peer: {:?}", maybe_peer);

            let peer = match maybe_peer {
                Ok(peer) => peer,
                Err(_) => continue,
            };

            let result = self.data_client.send_data_summary_request(peer).await;

            trace!("DataSummaryPoller: maybe received response: {:?}", result);

            let summary = match result {
                Ok(summary) => summary,
                Err(_err) => continue,
            };

            self.data_client.update_summary(peer, summary);
            self.data_client.update_global_summary_cache();
        }
    }
}

#[derive(Debug, Default)]
struct PeerState {
    storage_summary: Option<StorageServerSummary>,
    // TODO(philiphayes): imagine storing some scoring info here.
    metadata: (),
}

/// Contains all of the unbanned peers' most recent [`StorageServerSummary`] data
/// advertisements and data-client internal metadata for scoring.
#[derive(Debug)]
struct PeerStates {
    inner: HashMap<PeerNetworkId, PeerState>,
}

impl PeerStates {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Returns true if a connected storage service peer can actually fulfill a
    /// request, given our current view of their advertised data summary.
    fn can_service_request(&self, peer: &PeerNetworkId, request: &StorageServiceRequest) -> bool {
        // Storage services can always respond to data advertisement requests.
        // We need this outer check, since we need to be able to send data summary
        // requests to new peers (who don't have a peer state yet).
        if request.is_get_storage_server_summary() {
            return true;
        }

        self.inner
            .get(peer)
            .and_then(|peer_state| peer_state.storage_summary.as_ref())
            .map(|summary| summary.can_service(request))
            .unwrap_or(false)
    }

    fn update_summary(&mut self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.inner.entry(peer).or_default().storage_summary = Some(summary);
    }

    fn aggregate_summary(&self) -> GlobalDataSummary {
        let mut aggregate_data = AdvertisedData::empty();

        let mut max_epoch_chunk_sizes = vec![];
        let mut max_transaction_chunk_sizes = vec![];
        let mut max_transaction_output_chunk_sizes = vec![];
        let mut max_account_states_chunk_sizes = vec![];

        let summaries = self
            .inner
            .values()
            .filter_map(|state| state.storage_summary.as_ref());

        // collect each peer's protocol and data advertisements
        for summary in summaries {
            // collect aggregate data advertisements
            if let Some(account_states) = summary.data_summary.account_states {
                aggregate_data.account_states.push(account_states);
            }
            if let Some(epoch_ending_ledger_infos) = summary.data_summary.epoch_ending_ledger_infos
            {
                aggregate_data
                    .epoch_ending_ledger_infos
                    .push(epoch_ending_ledger_infos);
            }
            if let Some(synced_ledger_info) = summary.data_summary.synced_ledger_info.as_ref() {
                aggregate_data
                    .synced_ledger_infos
                    .push(synced_ledger_info.clone());
            }
            if let Some(transactions) = summary.data_summary.transactions {
                aggregate_data.transactions.push(transactions);
            }
            if let Some(transaction_outputs) = summary.data_summary.transaction_outputs {
                aggregate_data.transaction_outputs.push(transaction_outputs);
            }

            // collect preferred max chunk sizes
            max_epoch_chunk_sizes.push(summary.protocol_metadata.max_epoch_chunk_size);
            max_transaction_chunk_sizes.push(summary.protocol_metadata.max_transaction_chunk_size);
            max_transaction_output_chunk_sizes
                .push(summary.protocol_metadata.max_transaction_output_chunk_size);
            max_account_states_chunk_sizes
                .push(summary.protocol_metadata.max_account_states_chunk_size);
        }

        // take the median for each max chunk size parameter.
        // this works well when we have an honest majority that mostly agrees on
        // the same chunk sizes.
        // TODO(philiphayes): move these constants somewhere?
        let aggregate_chunk_sizes = OptimalChunkSizes {
            account_states_chunk_size: median(&mut max_account_states_chunk_sizes)
                .unwrap_or(storage_service_server::MAX_ACCOUNT_STATES_CHUNK_SIZE),
            epoch_chunk_size: median(&mut max_epoch_chunk_sizes)
                .unwrap_or(storage_service_server::MAX_EPOCH_CHUNK_SIZE),
            transaction_chunk_size: median(&mut max_transaction_chunk_sizes)
                .unwrap_or(storage_service_server::MAX_TRANSACTION_CHUNK_SIZE),
            transaction_output_chunk_size: median(&mut max_transaction_output_chunk_sizes)
                .unwrap_or(storage_service_server::MAX_TRANSACTION_OUTPUT_CHUNK_SIZE),
        };

        GlobalDataSummary {
            advertised_data: aggregate_data,
            optimal_chunk_sizes: aggregate_chunk_sizes,
        }
    }
}

fn median<T: Ord + Copy>(values: &mut [T]) -> Option<T> {
    values.sort_unstable();
    let idx = values.len() / 2;
    values.get(idx).copied()
}
