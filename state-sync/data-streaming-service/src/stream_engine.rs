// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification::{
        DataClientRequest,
        DataClientRequest::{
            EpochEndingLedgerInfos, NewTransactionOutputsWithProof,
            NewTransactionsOrOutputsWithProof, NewTransactionsWithProof, NumberOfStates,
            StateValuesWithProof, SubscribeTransactionOutputsWithProof,
            SubscribeTransactionsOrOutputsWithProof, SubscribeTransactionsWithProof,
            TransactionOutputsWithProof, TransactionsOrOutputsWithProof, TransactionsWithProof,
        },
        DataNotification, DataPayload, EpochEndingLedgerInfosRequest,
        NewTransactionOutputsWithProofRequest, NewTransactionsOrOutputsWithProofRequest,
        NewTransactionsWithProofRequest, NumberOfStatesRequest, StateValuesWithProofRequest,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    streaming_client::{
        Epoch, GetAllEpochEndingLedgerInfosRequest, GetAllStatesRequest, StreamRequest,
    },
};
use aptos_config::config::DataStreamingServiceConfig;
use aptos_data_client::{
    global_summary::{AdvertisedData, GlobalDataSummary},
    interface::ResponsePayload,
};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_logger::prelude::*;
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use enum_dispatch::enum_dispatch;
use std::{cmp, cmp::min, sync::Arc};

macro_rules! invalid_client_request {
    ($client_request:expr, $stream_engine:expr) => {
        return Err(Error::UnexpectedErrorEncountered(format!(
            "Invalid client request {:?} found for the data stream engine {:?}",
            $client_request, $stream_engine
        )))
    };
}

macro_rules! invalid_response_type {
    ($client_response:expr) => {
        return Err(Error::UnexpectedErrorEncountered(format!(
            "The client response is type mismatched: {:?}",
            $client_response
        )))
    };
}

macro_rules! invalid_stream_request {
    ($stream_request:expr) => {
        return Err(Error::UnexpectedErrorEncountered(format!(
            "Invalid stream request found {:?}",
            format!("{:?}", $stream_request)
        )))
    };
}

/// The interface offered by each data stream engine.
#[enum_dispatch]
pub trait DataStreamEngine {
    /// Creates a batch of data client requests that can be sent to the
    /// Aptos data client to progress the stream. The number of requests
    /// created is bound by the `max_number_of_requests`.
    fn create_data_client_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
        global_data_summary: &GlobalDataSummary,
        unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Vec<DataClientRequest>, Error>;

    /// Returns true iff all remaining data required to satisfy the stream is
    /// available in the given advertised data.
    fn is_remaining_data_available(&self, advertised_data: &AdvertisedData) -> Result<bool, Error>;

    /// Returns true iff the stream has sent all data to the stream listener.
    fn is_stream_complete(&self) -> bool;

    /// Notifies the data stream engine that an error was encountered when
    /// trying to send an optimistic fetch or subscription request.
    ///
    /// Note: Most engines shouldn't process these notifications, so a
    /// default implementation that returns an error is provided.
    fn notify_new_data_request_error(
        &mut self,
        client_request: &DataClientRequest,
        request_error: aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        Err(Error::UnexpectedErrorEncountered(format!(
            "Received a new data request error notification but no request was sent! Reported error: {:?}, request: {:?}",
            request_error, client_request
        )))
    }

    /// Transforms a given data client response (for the previously sent
    /// request) into a data notification to be sent along the data stream.
    /// Note: this call may return `None`, in which case, no notification needs
    /// to be sent to the client.
    fn transform_client_response_into_notification(
        &mut self,
        client_request: &DataClientRequest,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Option<DataNotification>, Error>;
}

/// Different types of data stream engines that allow each data stream to
/// track and update progress through the `DataStreamEngine` interface.
#[enum_dispatch(DataStreamEngine)]
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum StreamEngine {
    ContinuousTransactionStreamEngine,
    EpochEndingStreamEngine,
    StateStreamEngine,
    TransactionStreamEngine,
}

impl StreamEngine {
    pub fn new(
        data_stream_config: DataStreamingServiceConfig,
        stream_request: &StreamRequest,
        advertised_data: &AdvertisedData,
    ) -> Result<Self, Error> {
        match stream_request {
            StreamRequest::ContinuouslyStreamTransactionOutputs(_) => Ok(
                ContinuousTransactionStreamEngine::new(data_stream_config, stream_request)?.into(),
            ),
            StreamRequest::ContinuouslyStreamTransactions(_) => Ok(
                ContinuousTransactionStreamEngine::new(data_stream_config, stream_request)?.into(),
            ),
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => Ok(
                ContinuousTransactionStreamEngine::new(data_stream_config, stream_request)?.into(),
            ),
            StreamRequest::GetAllStates(request) => Ok(StateStreamEngine::new(request)?.into()),
            StreamRequest::GetAllEpochEndingLedgerInfos(request) => {
                Ok(EpochEndingStreamEngine::new(request, advertised_data)?.into())
            },
            StreamRequest::GetAllTransactionOutputs(_) => {
                Ok(TransactionStreamEngine::new(stream_request)?.into())
            },
            StreamRequest::GetAllTransactions(_) => {
                Ok(TransactionStreamEngine::new(stream_request)?.into())
            },
            StreamRequest::GetAllTransactionsOrOutputs(_) => {
                Ok(TransactionStreamEngine::new(stream_request)?.into())
            },
            _ => Err(Error::UnsupportedRequestEncountered(format!(
                "Stream request not supported: {:?}",
                stream_request
            ))),
        }
    }
}

#[derive(Clone, Debug)]
pub struct StateStreamEngine {
    // The original states request made by the client
    pub request: GetAllStatesRequest,

    // True iff a request has been created to fetch the number of states
    pub state_num_requested: bool,

    // The total number of states to fetch at this version
    pub number_of_states: Option<u64>,

    // The next state index that we're waiting to send to the client along the
    // stream. All states before this index have already been sent.
    pub next_stream_index: u64,

    // The next state index that we're waiting to request from the network.
    // All states before this index have already been requested.
    pub next_request_index: u64,

    // True iff all data has been sent across the stream.
    pub stream_is_complete: bool,
}

impl StateStreamEngine {
    fn new(request: &GetAllStatesRequest) -> Result<Self, Error> {
        Ok(StateStreamEngine {
            request: request.clone(),
            state_num_requested: false,
            number_of_states: None,
            next_stream_index: request.start_index,
            next_request_index: request.start_index,
            stream_is_complete: false,
        })
    }

    fn update_request_tracking(
        &mut self,
        client_requests: &[DataClientRequest],
    ) -> Result<(), Error> {
        for client_request in client_requests {
            match client_request {
                StateValuesWithProof(request) => {
                    self.next_request_index =
                        request.end_index.checked_add(1).ok_or_else(|| {
                            Error::IntegerOverflow("Next request index has overflown!".into())
                        })?;
                },
                request => invalid_client_request!(request, self),
            }
        }
        Ok(())
    }

    fn get_number_of_states(&self) -> Result<u64, Error> {
        self.number_of_states.ok_or_else(|| {
            Error::UnexpectedErrorEncountered("Number of states is not initialized!".into())
        })
    }
}

impl DataStreamEngine for StateStreamEngine {
    fn create_data_client_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
        global_data_summary: &GlobalDataSummary,
        _unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Vec<DataClientRequest>, Error> {
        // Check if we should wait for the number of states to be returned
        if self.number_of_states.is_none() && self.state_num_requested {
            return Ok(vec![]);
        }

        // If we have the number of states, send the requests
        if let Some(number_of_states) = self.number_of_states {
            // Calculate the number of requests to send
            let num_requests_to_send = calculate_num_requests_to_send(
                max_number_of_requests,
                max_in_flight_requests,
                num_in_flight_requests,
            );

            // Calculate the end index
            let end_state_index = number_of_states
                .checked_sub(1)
                .ok_or_else(|| Error::IntegerOverflow("End state index has overflown!".into()))?;

            // Create the client requests
            let client_requests = create_data_client_request_batch(
                self.next_request_index,
                end_state_index,
                num_requests_to_send,
                global_data_summary.optimal_chunk_sizes.state_chunk_size,
                self.clone().into(),
            )?;

            // Return the requests
            self.update_request_tracking(&client_requests)?;
            return Ok(client_requests);
        }

        // Otherwise, we need to request the number of states
        info!(
            (LogSchema::new(LogEntry::AptosDataClient)
                .event(LogEvent::Pending)
                .message(&format!(
                    "Requested the number of states at version: {:?}",
                    self.request.version
                )))
        );

        // Return the request
        self.state_num_requested = true;
        Ok(vec![DataClientRequest::NumberOfStates(
            NumberOfStatesRequest {
                version: self.request.version,
            },
        )])
    }

    fn is_remaining_data_available(&self, advertised_data: &AdvertisedData) -> Result<bool, Error> {
        Ok(AdvertisedData::contains_range(
            self.request.version,
            self.request.version,
            &advertised_data.states,
        ))
    }

    fn is_stream_complete(&self) -> bool {
        self.stream_is_complete
    }

    fn transform_client_response_into_notification(
        &mut self,
        client_request: &DataClientRequest,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Option<DataNotification>, Error> {
        // Update the metrics for the number of received items
        update_response_chunk_size_metrics(client_request, &client_response_payload);

        // Handle and transform the response
        match client_request {
            StateValuesWithProof(request) => {
                // Verify the client request indices
                verify_client_request_indices(
                    self.next_stream_index,
                    request.start_index,
                    request.end_index,
                )?;

                // Identify the last received state index and bound it appropriately
                let last_received_index = match &client_response_payload {
                    ResponsePayload::StateValuesWithProof(state_values_with_proof) => {
                        // Verify that we received at least one state value
                        if state_values_with_proof.raw_values.is_empty() {
                            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                                "Received an empty state values response! Request: {:?}",
                                client_request
                            )));
                        }

                        // Get the last received state index
                        state_values_with_proof.last_index
                    },
                    _ => invalid_response_type!(client_response_payload),
                };
                let last_received_index =
                    bound_by_range(last_received_index, request.start_index, request.end_index);

                // Update the next stream index
                self.next_stream_index = last_received_index.checked_add(1).ok_or_else(|| {
                    Error::IntegerOverflow("Next stream index has overflown!".into())
                })?;

                // Check if the stream is complete
                let last_stream_index = self
                    .get_number_of_states()?
                    .checked_sub(1)
                    .ok_or_else(|| Error::IntegerOverflow("End index has overflown!".into()))?;
                if last_received_index >= last_stream_index {
                    self.stream_is_complete = true;
                }

                // Create a new data notification
                let data_notification = create_data_notification(
                    notification_id_generator,
                    client_response_payload,
                    None,
                    self.clone().into(),
                )?;
                return Ok(Some(data_notification));
            },
            NumberOfStates(request) => {
                if let ResponsePayload::NumberOfStates(number_of_states) = client_response_payload {
                    info!(
                        (LogSchema::new(LogEntry::ReceivedDataResponse)
                            .event(LogEvent::Success)
                            .message(&format!(
                                "Received number of states at version: {:?}. Total states: {:?}",
                                request.version, number_of_states
                            )))
                    );
                    self.state_num_requested = false;

                    // Sanity check the response before saving it.
                    if number_of_states < self.next_request_index {
                        return Err(Error::NoDataToFetch(format!(
                            "The next state index to fetch is higher than the \
                            total number of states. Next index: {:?}, total states: {:?}",
                            self.next_request_index, number_of_states
                        )));
                    } else {
                        self.number_of_states = Some(number_of_states);
                    }
                }
            },
            request => invalid_client_request!(request, self),
        }
        Ok(None)
    }
}

#[derive(Clone, Debug)]
pub struct ContinuousTransactionStreamEngine {
    // The data streaming service config
    pub data_streaming_config: DataStreamingServiceConfig,

    // The original stream request made by the client (i.e., a continuous
    // transaction or transaction output stream request).
    pub request: StreamRequest,

    // The target ledger info that we're currently syncing to
    pub current_target_ledger_info: Option<LedgerInfoWithSignatures>,

    // True iff a request has been created to fetch an epoch ending ledger info
    pub end_of_epoch_requested: bool,

    // True iff a request has been created to optimistically fetch data
    pub optimistic_fetch_requested: bool,

    // The active subscription stream (if it exists)
    active_subscription_stream: Option<SubscriptionStream>,

    // The next version and epoch that we're waiting to send to the
    // client along the stream. All versions before this have been sent.
    pub next_stream_version_and_epoch: (Version, Epoch),

    // The next version and epoch that we're waiting to request from
    // the network. All versions before this have been requested.
    pub next_request_version_and_epoch: (Version, Epoch),

    // True iff all data has been sent across the stream. This will only be
    // possible if there is a target ledger info specified.
    pub stream_is_complete: bool,
}

impl ContinuousTransactionStreamEngine {
    fn new(
        data_streaming_config: DataStreamingServiceConfig,
        stream_request: &StreamRequest,
    ) -> Result<Self, Error> {
        let (next_version, next_epoch) = match stream_request {
            StreamRequest::ContinuouslyStreamTransactions(request) => {
                Self::calculate_next_version_and_epoch(request.known_version, request.known_epoch)?
            },
            StreamRequest::ContinuouslyStreamTransactionOutputs(request) => {
                Self::calculate_next_version_and_epoch(request.known_version, request.known_epoch)?
            },
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                Self::calculate_next_version_and_epoch(request.known_version, request.known_epoch)?
            },
            request => invalid_stream_request!(request),
        };

        Ok(ContinuousTransactionStreamEngine {
            data_streaming_config,
            request: stream_request.clone(),
            current_target_ledger_info: None,
            end_of_epoch_requested: false,
            optimistic_fetch_requested: false,
            active_subscription_stream: None,
            next_stream_version_and_epoch: (next_version, next_epoch),
            next_request_version_and_epoch: (next_version, next_epoch),
            stream_is_complete: false,
        })
    }

    fn calculate_next_version_and_epoch(
        known_version: Version,
        known_epoch: Epoch,
    ) -> Result<(Version, Epoch), Error> {
        let next_version = known_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next version has overflown!".into()))?;
        Ok((next_version, known_epoch))
    }

    fn select_target_ledger_info(
        &self,
        advertised_data: &AdvertisedData,
    ) -> Result<Option<LedgerInfoWithSignatures>, Error> {
        // Check if the stream has a final target ledger info
        match &self.request {
            StreamRequest::ContinuouslyStreamTransactions(request) => {
                if let Some(target) = &request.target {
                    return Ok(Some(target.clone()));
                }
            },
            StreamRequest::ContinuouslyStreamTransactionOutputs(request) => {
                if let Some(target) = &request.target {
                    return Ok(Some(target.clone()));
                }
            },
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                if let Some(target) = &request.target {
                    return Ok(Some(target.clone()));
                }
            },
            request => invalid_stream_request!(request),
        };

        // We don't have a final target, select the highest to make progress
        if let Some(highest_synced_ledger_info) = advertised_data.highest_synced_ledger_info() {
            let (next_request_version, _) = self.next_request_version_and_epoch;
            if next_request_version > highest_synced_ledger_info.ledger_info().version() {
                Ok(None) // We're already at the highest synced ledger info. There's no known target.
            } else {
                Ok(Some(highest_synced_ledger_info))
            }
        } else {
            Err(Error::DataIsUnavailable(
                "Unable to find the highest synced ledger info!".into(),
            ))
        }
    }

    fn get_target_ledger_info(&self) -> Result<&LedgerInfoWithSignatures, Error> {
        self.current_target_ledger_info.as_ref().ok_or_else(|| {
            Error::UnexpectedErrorEncountered("No current target ledger info found!".into())
        })
    }

    fn create_notification_for_continuous_data(
        &mut self,
        request_start_version: Version,
        request_end_version: Version,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<DataNotification, Error> {
        // Check the number of received versions
        let num_received_versions = match &client_response_payload {
            ResponsePayload::TransactionsWithProof(transactions_with_proof) => {
                transactions_with_proof.transactions.len()
            },
            ResponsePayload::TransactionOutputsWithProof(outputs_with_proof) => {
                outputs_with_proof.transactions_and_outputs.len()
            },
            _ => invalid_response_type!(client_response_payload),
        };
        if num_received_versions == 0 {
            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                "Received an empty continuous data response! Request: {:?}",
                self.request
            )));
        }

        // Identify the last received version and bound it appropriately
        let last_received_version = request_start_version
            .checked_add(num_received_versions as u64)
            .and_then(|version| version.checked_sub(1))
            .ok_or_else(|| Error::IntegerOverflow("Last received version has overflown!".into()))?;
        let last_received_version = bound_by_range(
            last_received_version,
            request_start_version,
            request_end_version,
        );

        // Update the stream version
        let target_ledger_info = self.get_target_ledger_info()?.clone();
        self.update_stream_version_and_epoch(
            request_start_version,
            request_end_version,
            &target_ledger_info,
            last_received_version,
        )?;

        // Create the data notification
        let data_notification = create_data_notification(
            notification_id_generator,
            client_response_payload,
            Some(target_ledger_info),
            self.clone().into(),
        )?;
        Ok(data_notification)
    }

    /// Creates a data notification for new transaction data
    /// starting at the specified first version.
    fn create_notification_for_new_data(
        &mut self,
        first_version: u64,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<DataNotification, Error> {
        // Calculate the number of data items and target ledger info
        let (num_versions, target_ledger_info) =
            extract_new_versions_and_target(&client_response_payload)?;

        // Calculate the last version (last_version = first_version + num_versions - 1)
        let last_version = first_version
            .checked_add(num_versions as u64)
            .and_then(|v| v.checked_sub(1))
            .ok_or_else(|| Error::IntegerOverflow("Last version has overflown!".into()))?;

        // Update the request and stream versions
        self.update_request_version_and_epoch(last_version, &target_ledger_info)?;
        self.update_stream_version_and_epoch(
            first_version,
            last_version,
            &target_ledger_info,
            last_version,
        )?;

        // Create the data notification
        let data_notification = create_data_notification(
            notification_id_generator,
            client_response_payload,
            Some(target_ledger_info.clone()),
            self.clone().into(),
        )?;

        Ok(data_notification)
    }

    fn create_notification_for_optimistic_fetch_data(
        &mut self,
        known_version: Version,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<DataNotification, Error> {
        // Calculate the first version
        let first_version = known_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("First version has overflown!".into()))?;

        // Create the data notification
        self.create_notification_for_new_data(
            first_version,
            client_response_payload,
            notification_id_generator,
        )
    }

    /// Creates a notification for subscription data by
    /// transforming the given client response payload.
    fn create_notification_for_subscription_data(
        &mut self,
        subscription_stream_index: u64,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<DataNotification, Error> {
        // If there's an active subscription and this is the
        // last expected response then terminate the stream.
        if let Some(active_subscription_stream) = &self.active_subscription_stream {
            if subscription_stream_index
                >= active_subscription_stream.get_max_subscription_stream_index()
            {
                // Terminate the stream and update the termination metrics
                self.active_subscription_stream = None;
                update_terminated_subscription_metrics(metrics::MAX_CONSECUTIVE_REQUESTS_LABEL);
            }
        }

        // Get the first version
        let (first_version, _) = self.next_request_version_and_epoch;

        // Create the data notification
        self.create_notification_for_new_data(
            first_version,
            client_response_payload,
            notification_id_generator,
        )
    }

    /// Creates an optimistic fetch request for the current stream state
    fn create_optimistic_fetch_request(&mut self) -> Result<DataClientRequest, Error> {
        let (known_version, known_epoch) = self.get_known_version_and_epoch()?;
        let data_client_request = match &self.request {
            StreamRequest::ContinuouslyStreamTransactions(request) => {
                NewTransactionsWithProof(NewTransactionsWithProofRequest {
                    known_version,
                    known_epoch,
                    include_events: request.include_events,
                })
            },
            StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                NewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                    known_version,
                    known_epoch,
                })
            },
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                NewTransactionsOrOutputsWithProof(NewTransactionsOrOutputsWithProofRequest {
                    known_version,
                    known_epoch,
                    include_events: request.include_events,
                })
            },
            request => invalid_stream_request!(request),
        };

        Ok(data_client_request)
    }

    /// Creates a new set of subscription stream requests
    /// to extend the currently active subscription stream.
    fn create_subscription_stream_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
    ) -> Result<Vec<DataClientRequest>, Error> {
        // Get the active subscription stream
        let mut active_subscription_stream = match self.active_subscription_stream.take() {
            Some(active_subscription_stream) => active_subscription_stream,
            None => {
                // We don't have an active subscription stream!
                return Err(Error::UnexpectedErrorEncountered(
                    "No active subscription stream found! Unable to create requests!".into(),
                ));
            },
        };

        // Get the highest known version and epoch at stream start
        let (known_version, known_epoch) =
            active_subscription_stream.get_known_version_and_epoch_at_stream_start();

        // TODO(joshlind): identify a way to avoid overriding this here.
        // Determine the maximum number of in-flight requests. This is overridden
        // if dynamic prefetching is enabled (to avoid making too few/many subscriptions).
        let prefetching_config = &self.data_streaming_config.dynamic_prefetching;
        let max_in_flight_requests = if prefetching_config.enable_dynamic_prefetching {
            // Use the max number of in-flight subscriptions from the prefetching config
            prefetching_config.max_in_flight_subscription_requests
        } else {
            max_in_flight_requests // Otherwise, use the given maximum
        };

        // Calculate the number of requests to send
        let num_requests_to_send = calculate_num_requests_to_send(
            max_number_of_requests,
            max_in_flight_requests,
            num_in_flight_requests,
        );

        // Create the subscription stream requests
        let mut subscription_stream_requests = vec![];
        for _ in 0..num_requests_to_send {
            // Get the current subscription stream ID and index
            let subscription_stream_id = active_subscription_stream.get_subscription_stream_id();
            let subscription_stream_index =
                active_subscription_stream.get_next_subscription_stream_index();

            // Note: if the stream hits the total max subscription stream index,
            // then no new requests should be created. The stream will eventually
            // be terminated once a response is received for the last request.
            if subscription_stream_index
                > active_subscription_stream.get_max_subscription_stream_index()
            {
                break;
            }

            // Create the request based on the stream type
            let data_client_request = match &self.request {
                StreamRequest::ContinuouslyStreamTransactions(request) => {
                    SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
                        known_version,
                        known_epoch,
                        include_events: request.include_events,
                        subscription_stream_id,
                        subscription_stream_index,
                    })
                },
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    SubscribeTransactionOutputsWithProof(
                        SubscribeTransactionOutputsWithProofRequest {
                            known_version,
                            known_epoch,
                            subscription_stream_id,
                            subscription_stream_index,
                        },
                    )
                },
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                    SubscribeTransactionsOrOutputsWithProof(
                        SubscribeTransactionsOrOutputsWithProofRequest {
                            known_version,
                            known_epoch,
                            include_events: request.include_events,
                            subscription_stream_id,
                            subscription_stream_index,
                        },
                    )
                },
                request => invalid_stream_request!(request),
            };

            // Update the next subscription stream index
            active_subscription_stream.increment_subscription_stream_index();

            // Add the request to the active list
            subscription_stream_requests.push(data_client_request);
        }

        // Update the active subscription stream state
        self.active_subscription_stream = Some(active_subscription_stream);

        // Return the subscription stream requests
        Ok(subscription_stream_requests)
    }

    fn handle_epoch_ending_response(
        &mut self,
        response_payload: ResponsePayload,
    ) -> Result<(), Error> {
        if let ResponsePayload::EpochEndingLedgerInfos(epoch_ending_ledger_infos) = response_payload
        {
            match &epoch_ending_ledger_infos[..] {
                [target_ledger_info] => {
                    info!(
                        (LogSchema::new(LogEntry::ReceivedDataResponse)
                            .event(LogEvent::Success)
                            .message(&format!(
                                "Received an epoch ending ledger info for epoch: {:?}. \
                                        Setting new target version: {:?}",
                                target_ledger_info.ledger_info().epoch(),
                                target_ledger_info.ledger_info().version()
                            )))
                    );
                    self.current_target_ledger_info = Some(target_ledger_info.clone());
                    Ok(())
                },
                response_payload => {
                    // TODO(joshlind): eventually we want to notify the data client of the bad response
                    Err(Error::AptosDataClientResponseIsInvalid(format!(
                        "Received an incorrect number of epoch ending ledger infos. Response: {:?}",
                        response_payload
                    )))
                },
            }
        } else {
            // TODO(joshlind): eventually we want to notify the data client of the bad response
            Err(Error::AptosDataClientResponseIsInvalid(format!(
                "Expected an epoch ending ledger response but got: {:?}",
                response_payload
            )))
        }
    }

    /// Returns the known version and epoch for the stream
    fn get_known_version_and_epoch(&mut self) -> Result<(u64, Epoch), Error> {
        let (next_request_version, known_epoch) = self.next_request_version_and_epoch;
        let known_version = next_request_version
            .checked_sub(1)
            .ok_or_else(|| Error::IntegerOverflow("Last version has overflown!".into()))?;

        Ok((known_version, known_epoch))
    }

    /// Handles an optimistic fetch timeout for the specified client request
    fn handle_optimistic_fetch_error(
        &mut self,
        client_request: &DataClientRequest,
        request_error: aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        // We should only receive an error notification if we sent an optimistic fetch request
        if !self.optimistic_fetch_requested {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Received an optimistic fetch notification error but no request is in-flight! Error: {:?}, request: {:?}",
                request_error, client_request
            )));
        }

        // Reset the optimistic fetch request
        self.optimistic_fetch_requested = false;

        // Log the error based on the request type
        if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactions(_)
        ) && matches!(
            client_request,
            DataClientRequest::NewTransactionsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Optimistic fetch error for new transactions: {:?}",
                    request_error
                )))
            );
        } else if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactionOutputs(_)
        ) && matches!(
            client_request,
            DataClientRequest::NewTransactionOutputsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Optimistic fetch error for new transaction outputs: {:?}",
                    request_error
                )))
            );
        } else if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_)
        ) && matches!(
            client_request,
            DataClientRequest::NewTransactionsOrOutputsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Optimistic fetch error for new transactions or outputs: {:?}",
                    request_error
                )))
            );
        } else {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Received an optimistic fetch error but the request did not match the expected type for the stream! \
                Error: {:?}, request: {:?}, stream: {:?}", request_error, client_request, self.request
            )));
        }

        Ok(())
    }

    /// Handles a subscription error for the specified client request
    fn handle_subscription_error(
        &mut self,
        client_request: &DataClientRequest,
        request_error: aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        // We should only receive an error notification if we have an active stream
        if self.active_subscription_stream.is_none() {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Received a subscription notification error but no active subscription stream exists! Error: {:?}, request: {:?}",
                request_error, client_request
            )));
        }

        // Reset the active subscription stream and update the metrics
        self.active_subscription_stream = None;
        update_terminated_subscription_metrics(request_error.get_label());

        // Log the error based on the request type
        if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactions(_)
        ) && matches!(
            client_request,
            DataClientRequest::SubscribeTransactionsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Subscription error for new transactions: {:?}",
                    request_error
                )))
            );
        } else if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactionOutputs(_)
        ) && matches!(
            client_request,
            DataClientRequest::SubscribeTransactionOutputsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Subscription error for new transaction outputs: {:?}",
                    request_error
                )))
            );
        } else if matches!(
            self.request,
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_)
        ) && matches!(
            client_request,
            DataClientRequest::SubscribeTransactionsOrOutputsWithProof(_)
        ) {
            info!(
                (LogSchema::new(LogEntry::RequestError).message(&format!(
                    "Subscription error for new transactions or outputs: {:?}",
                    request_error
                )))
            );
        } else {
            return Err(Error::UnexpectedErrorEncountered(format!(
                "Received a subscription request error but the request did not match the expected type for the stream! \
                Error: {:?}, request: {:?}, stream: {:?}", request_error, client_request, self.request
            )));
        }

        Ok(())
    }

    /// Starts a new active subscription stream
    fn start_active_subscription_stream(
        &mut self,
        unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<(), Error> {
        // Verify that we don't already have an active subscription stream
        if self.active_subscription_stream.is_some() {
            return Err(Error::UnexpectedErrorEncountered(
                "Unable to start a new subscription stream when one is already active!".into(),
            ));
        }

        // Get the highest known version and epoch
        let (known_version, known_epoch) = self.get_known_version_and_epoch()?;

        // Create and save a new subscription stream
        let subscription_stream = SubscriptionStream::new(
            self.data_streaming_config,
            unique_id_generator,
            known_version,
            known_epoch,
        );
        self.active_subscription_stream = Some(subscription_stream);

        // Update the metrics counter
        metrics::CREATE_SUBSCRIPTION_STREAM.inc();

        Ok(())
    }

    fn update_stream_version_and_epoch(
        &mut self,
        request_start_version: Version,
        request_end_version: Version,
        target_ledger_info: &LedgerInfoWithSignatures,
        last_received_version: Version,
    ) -> Result<(), Error> {
        // Verify the client request indices
        let (next_stream_version, mut next_stream_epoch) = self.next_stream_version_and_epoch;
        verify_client_request_indices(
            next_stream_version,
            request_start_version,
            request_end_version,
        )?;

        // Update the next stream version and epoch
        if last_received_version == target_ledger_info.ledger_info().version()
            && target_ledger_info.ledger_info().ends_epoch()
        {
            next_stream_epoch = next_stream_epoch
                .checked_add(1)
                .ok_or_else(|| Error::IntegerOverflow("Next stream epoch has overflown!".into()))?;
        }
        let next_stream_version = last_received_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next stream version has overflown!".into()))?;
        self.next_stream_version_and_epoch = (next_stream_version, next_stream_epoch);

        // Check if the stream is now complete
        let stream_request_target = match &self.request {
            StreamRequest::ContinuouslyStreamTransactions(request) => request.target.clone(),
            StreamRequest::ContinuouslyStreamTransactionOutputs(request) => request.target.clone(),
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                request.target.clone()
            },
            request => invalid_stream_request!(request),
        };
        if let Some(target) = stream_request_target {
            if last_received_version >= target.ledger_info().version() {
                self.stream_is_complete = true;
            }
        }

        // Update the current target ledger info if we've hit it
        if last_received_version >= target_ledger_info.ledger_info().version() {
            self.current_target_ledger_info = None;
        }

        Ok(())
    }

    fn update_request_version_and_epoch(
        &mut self,
        request_end_version: Version,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        // Calculate the next request epoch
        let (_, mut next_request_epoch) = self.next_request_version_and_epoch;
        if request_end_version == target_ledger_info.ledger_info().version()
            && target_ledger_info.ledger_info().ends_epoch()
        {
            // We've hit an epoch change
            next_request_epoch = next_request_epoch.checked_add(1).ok_or_else(|| {
                Error::IntegerOverflow("Next request epoch has overflown!".into())
            })?;
        }

        // Update the next request version and epoch
        let next_request_version = request_end_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next request version has overflown!".into()))?;
        self.next_request_version_and_epoch = (next_request_version, next_request_epoch);

        Ok(())
    }

    fn update_request_tracking(
        &mut self,
        client_requests: &[DataClientRequest],
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> Result<(), Error> {
        match &self.request {
            StreamRequest::ContinuouslyStreamTransactions(_) => {
                for client_request in client_requests {
                    match client_request {
                        DataClientRequest::TransactionsWithProof(request) => {
                            self.update_request_version_and_epoch(
                                request.end_version,
                                target_ledger_info,
                            )?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                for client_request in client_requests {
                    match client_request {
                        DataClientRequest::TransactionOutputsWithProof(request) => {
                            self.update_request_version_and_epoch(
                                request.end_version,
                                target_ledger_info,
                            )?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                for client_request in client_requests {
                    match client_request {
                        DataClientRequest::TransactionsOrOutputsWithProof(request) => {
                            self.update_request_version_and_epoch(
                                request.end_version,
                                target_ledger_info,
                            )?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            request => invalid_stream_request!(request),
        }

        Ok(())
    }
}

impl DataStreamEngine for ContinuousTransactionStreamEngine {
    fn create_data_client_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
        global_data_summary: &GlobalDataSummary,
        unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Vec<DataClientRequest>, Error> {
        // Check if we're waiting for a blocking response type
        if self.end_of_epoch_requested || self.optimistic_fetch_requested {
            return Ok(vec![]);
        }

        // If there's an active subscription stream we should utilize it
        if self.active_subscription_stream.is_some() {
            return self.create_subscription_stream_requests(
                max_number_of_requests,
                max_in_flight_requests,
                num_in_flight_requests,
            );
        }

        // If we don't have a syncing target, try to select one
        let (next_request_version, next_request_epoch) = self.next_request_version_and_epoch;
        if self.current_target_ledger_info.is_none() {
            // Try to select a new ledger info from the advertised data
            if let Some(target_ledger_info) =
                self.select_target_ledger_info(&global_data_summary.advertised_data)?
            {
                if target_ledger_info.ledger_info().epoch() > next_request_epoch {
                    // There was an epoch change. Request an epoch ending ledger info.
                    info!(
                        (LogSchema::new(LogEntry::AptosDataClient)
                            .event(LogEvent::Pending)
                            .message(&format!(
                                "Requested an epoch ending ledger info for epoch: {:?}",
                                next_request_epoch
                            )))
                    );
                    self.end_of_epoch_requested = true;
                    return Ok(vec![DataClientRequest::EpochEndingLedgerInfos(
                        EpochEndingLedgerInfosRequest {
                            start_epoch: next_request_epoch,
                            end_epoch: next_request_epoch,
                        },
                    )]);
                } else {
                    debug!(
                        (LogSchema::new(LogEntry::ReceivedDataResponse)
                            .event(LogEvent::Success)
                            .message(&format!(
                                "Setting new target ledger info. Version: {:?}, Epoch: {:?}",
                                target_ledger_info.ledger_info().version(),
                                target_ledger_info.ledger_info().epoch()
                            )))
                    );
                    self.current_target_ledger_info = Some(target_ledger_info);
                }
            }
        }

        // Create the next set of data client requests
        let maybe_target_ledger_info = self.current_target_ledger_info.clone();
        let client_requests = if let Some(target_ledger_info) = maybe_target_ledger_info {
            // Check if we're still waiting for stream notifications to be sent
            if next_request_version > target_ledger_info.ledger_info().version() {
                return Ok(vec![]);
            }

            // Calculate the number of requests to send
            let num_requests_to_send = calculate_num_requests_to_send(
                max_number_of_requests,
                max_in_flight_requests,
                num_in_flight_requests,
            );

            // Create the client requests for the target
            let optimal_chunk_sizes = match &self.request {
                StreamRequest::ContinuouslyStreamTransactions(_) => {
                    global_data_summary
                        .optimal_chunk_sizes
                        .transaction_chunk_size
                },
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    global_data_summary
                        .optimal_chunk_sizes
                        .transaction_output_chunk_size
                },
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                    global_data_summary
                        .optimal_chunk_sizes
                        .transaction_output_chunk_size
                },
                request => invalid_stream_request!(request),
            };
            let client_requests = create_data_client_request_batch(
                next_request_version,
                target_ledger_info.ledger_info().version(),
                num_requests_to_send,
                optimal_chunk_sizes,
                self.clone().into(),
            )?;
            self.update_request_tracking(&client_requests, &target_ledger_info)?;
            client_requests
        } else {
            // We don't have a target. We should either send an optimistic
            // fetch request or start a new subscription stream.
            if self.data_streaming_config.enable_subscription_streaming {
                // Start a new subscription stream and send the first set of requests
                self.start_active_subscription_stream(unique_id_generator)?;
                self.create_subscription_stream_requests(
                    max_number_of_requests,
                    max_in_flight_requests,
                    num_in_flight_requests,
                )?
            } else {
                // Send a single optimistic fetch request
                let optimistic_fetch_request = self.create_optimistic_fetch_request()?;
                self.optimistic_fetch_requested = true;
                vec![optimistic_fetch_request]
            }
        };

        // Return the requests
        Ok(client_requests)
    }

    fn is_remaining_data_available(&self, advertised_data: &AdvertisedData) -> Result<bool, Error> {
        let advertised_ranges = match &self.request {
            StreamRequest::ContinuouslyStreamTransactions(_) => &advertised_data.transactions,
            StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                &advertised_data.transaction_outputs
            },
            StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                &advertised_data.transaction_outputs
            },
            request => invalid_stream_request!(request),
        };

        // Verify we can satisfy the next version
        let (next_request_version, _) = self.next_request_version_and_epoch;
        Ok(AdvertisedData::contains_range(
            next_request_version,
            next_request_version,
            advertised_ranges,
        ))
    }

    fn is_stream_complete(&self) -> bool {
        self.stream_is_complete
    }

    fn notify_new_data_request_error(
        &mut self,
        client_request: &DataClientRequest,
        request_error: aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        // If subscription streaming is enabled, the timeout should be for
        // subscription data. Otherwise, it should be for optimistic fetch data.
        if self.data_streaming_config.enable_subscription_streaming {
            self.handle_subscription_error(client_request, request_error)
        } else {
            self.handle_optimistic_fetch_error(client_request, request_error)
        }
    }

    fn transform_client_response_into_notification(
        &mut self,
        client_request: &DataClientRequest,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Option<DataNotification>, Error> {
        // Reset the pending requests to prevent malicious responses from
        // blocking the streams. Note: these request types are mutually
        // exclusive and only a single request will exist at any given time.
        if self.end_of_epoch_requested {
            self.end_of_epoch_requested = false;
        } else if self.optimistic_fetch_requested {
            self.optimistic_fetch_requested = false;
        }

        // Update the metrics for the number of received items
        update_response_chunk_size_metrics(client_request, &client_response_payload);

        // Handle and transform the response
        match client_request {
            EpochEndingLedgerInfos(_) => {
                self.handle_epoch_ending_response(client_response_payload)?;
                Ok(None)
            },
            NewTransactionsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactions(_) => {
                    let data_notification = self.create_notification_for_optimistic_fetch_data(
                        request.known_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            NewTransactionOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    let data_notification = self.create_notification_for_optimistic_fetch_data(
                        request.known_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            NewTransactionsOrOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                    let data_notification = self.create_notification_for_optimistic_fetch_data(
                        request.known_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            SubscribeTransactionOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    let data_notification = self.create_notification_for_subscription_data(
                        request.subscription_stream_index,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            SubscribeTransactionsOrOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                    let data_notification = self.create_notification_for_subscription_data(
                        request.subscription_stream_index,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            SubscribeTransactionsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactions(_) => {
                    let data_notification = self.create_notification_for_subscription_data(
                        request.subscription_stream_index,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            TransactionsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactions(_) => {
                    let data_notification = self.create_notification_for_continuous_data(
                        request.start_version,
                        request.end_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            TransactionOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    let data_notification = self.create_notification_for_continuous_data(
                        request.start_version,
                        request.end_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            TransactionsOrOutputsWithProof(request) => match &self.request {
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(_) => {
                    let data_notification = self.create_notification_for_continuous_data(
                        request.start_version,
                        request.end_version,
                        client_response_payload,
                        notification_id_generator,
                    )?;
                    Ok(Some(data_notification))
                },
                request => invalid_stream_request!(request),
            },
            request => invalid_client_request!(request, self),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EpochEndingStreamEngine {
    // The original epoch ending ledger infos request made by the client
    #[allow(dead_code)]
    pub request: GetAllEpochEndingLedgerInfosRequest,

    // The last epoch ending ledger info that this stream will send to the client
    pub end_epoch: Epoch,

    // The next epoch that we're waiting to send to the client along the
    // stream. All epochs before this have already been sent.
    pub next_stream_epoch: Epoch,

    // The next epoch that we're waiting to request from the network. All epochs
    // before this have already been requested.
    pub next_request_epoch: Epoch,

    // True iff all data has been sent across the stream.
    pub stream_is_complete: bool,
}

impl EpochEndingStreamEngine {
    fn new(
        request: &GetAllEpochEndingLedgerInfosRequest,
        advertised_data: &AdvertisedData,
    ) -> Result<Self, Error> {
        let end_epoch = advertised_data
            .highest_epoch_ending_ledger_info()
            .ok_or_else(|| {
                Error::DataIsUnavailable(format!(
                    "Unable to find any epoch ending ledger info in the network: {:?}",
                    advertised_data
                ))
            })?;

        if end_epoch < request.start_epoch {
            return Err(Error::DataIsUnavailable(format!(
                "The epoch to start syncing from is higher than the highest epoch ending ledger info! Highest: {:?}, start: {:?}",
                end_epoch, request.start_epoch
            )));
        }
        info!(
            (LogSchema::new(LogEntry::ReceivedDataResponse)
                .event(LogEvent::Success)
                .message(&format!(
                    "Setting the highest epoch ending ledger info for the stream at: {:?}",
                    end_epoch
                )))
        );

        Ok(EpochEndingStreamEngine {
            request: request.clone(),
            end_epoch,
            next_stream_epoch: request.start_epoch,
            next_request_epoch: request.start_epoch,
            stream_is_complete: false,
        })
    }

    fn update_request_tracking(
        &mut self,
        client_requests: &[DataClientRequest],
    ) -> Result<(), Error> {
        for client_request in client_requests {
            match client_request {
                EpochEndingLedgerInfos(request) => {
                    self.next_request_epoch =
                        request.end_epoch.checked_add(1).ok_or_else(|| {
                            Error::IntegerOverflow("Next request epoch has overflown!".into())
                        })?;
                },
                request => invalid_client_request!(request, self),
            }
        }

        Ok(())
    }
}

impl DataStreamEngine for EpochEndingStreamEngine {
    fn create_data_client_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
        global_data_summary: &GlobalDataSummary,
        _unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Vec<DataClientRequest>, Error> {
        // Calculate the number of requests to send
        let num_requests_to_send = calculate_num_requests_to_send(
            max_number_of_requests,
            max_in_flight_requests,
            num_in_flight_requests,
        );

        // Create the client requests
        let client_requests = create_data_client_request_batch(
            self.next_request_epoch,
            self.end_epoch,
            num_requests_to_send,
            global_data_summary.optimal_chunk_sizes.epoch_chunk_size,
            self.clone().into(),
        )?;

        // Return the requests
        self.update_request_tracking(&client_requests)?;
        Ok(client_requests)
    }

    fn is_remaining_data_available(&self, advertised_data: &AdvertisedData) -> Result<bool, Error> {
        let start_epoch = self.next_stream_epoch;
        let end_epoch = self.end_epoch;
        Ok(AdvertisedData::contains_range(
            start_epoch,
            end_epoch,
            &advertised_data.epoch_ending_ledger_infos,
        ))
    }

    fn is_stream_complete(&self) -> bool {
        self.stream_is_complete
    }

    fn transform_client_response_into_notification(
        &mut self,
        client_request: &DataClientRequest,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Option<DataNotification>, Error> {
        // Update the metrics for the number of received items
        update_response_chunk_size_metrics(client_request, &client_response_payload);

        // Handle and transform the response
        match client_request {
            EpochEndingLedgerInfos(request) => {
                // Verify the client request indices
                verify_client_request_indices(
                    self.next_stream_epoch,
                    request.start_epoch,
                    request.end_epoch,
                )?;

                // Identify the last received epoch and bound it appropriately
                let last_received_epoch = match &client_response_payload {
                    ResponsePayload::EpochEndingLedgerInfos(ledger_infos) => {
                        // Verify that we received at least one ledger info
                        if ledger_infos.is_empty() {
                            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                                "Received an empty epoch ending ledger info response! Request: {:?}",
                                client_request
                            )));
                        }

                        // Return the last epoch
                        ledger_infos
                            .last()
                            .map(|ledger_info| ledger_info.ledger_info().epoch())
                            .unwrap_or(request.start_epoch)
                    },
                    _ => invalid_response_type!(client_response_payload),
                };
                let last_received_epoch =
                    bound_by_range(last_received_epoch, request.start_epoch, request.end_epoch);

                // Update the local stream notification tracker
                self.next_stream_epoch = last_received_epoch.checked_add(1).ok_or_else(|| {
                    Error::IntegerOverflow("Next stream epoch has overflown!".into())
                })?;

                // Check if the stream is complete
                if last_received_epoch >= self.end_epoch {
                    self.stream_is_complete = true;
                }

                // Create a new data notification
                let data_notification = create_data_notification(
                    notification_id_generator,
                    client_response_payload,
                    None,
                    self.clone().into(),
                )?;
                Ok(Some(data_notification))
            },
            request => invalid_client_request!(request, self),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransactionStreamEngine {
    // The original stream request made by the client (e.g., a transaction or
    // transaction output stream request).
    pub request: StreamRequest,

    // The next version that we're waiting to send to the client
    // along the stream. All versions before this have been sent.
    pub next_stream_version: Version,

    // The next version that we're waiting to request from the
    // network. All versions before this have already been requested.
    pub next_request_version: Version,

    // True iff all data has been sent across the stream.
    pub stream_is_complete: bool,
}

impl TransactionStreamEngine {
    fn new(stream_request: &StreamRequest) -> Result<Self, Error> {
        match stream_request {
            StreamRequest::GetAllTransactions(request) => Ok(TransactionStreamEngine {
                request: stream_request.clone(),
                next_stream_version: request.start_version,
                next_request_version: request.start_version,
                stream_is_complete: false,
            }),
            StreamRequest::GetAllTransactionOutputs(request) => Ok(TransactionStreamEngine {
                request: stream_request.clone(),
                next_stream_version: request.start_version,
                next_request_version: request.start_version,
                stream_is_complete: false,
            }),
            StreamRequest::GetAllTransactionsOrOutputs(request) => Ok(TransactionStreamEngine {
                request: stream_request.clone(),
                next_stream_version: request.start_version,
                next_request_version: request.start_version,
                stream_is_complete: false,
            }),
            request => invalid_stream_request!(request),
        }
    }

    fn update_stream_version(
        &mut self,
        request_start_version: Version,
        request_end_version: Version,
        stream_end_version: Version,
        client_response_payload: &ResponsePayload,
    ) -> Result<(), Error> {
        // Verify the client request indices
        verify_client_request_indices(
            self.next_stream_version,
            request_start_version,
            request_end_version,
        )?;

        // Check the number of received versions
        let num_received_versions = match client_response_payload {
            ResponsePayload::TransactionsWithProof(transactions_with_proof) => {
                transactions_with_proof.transactions.len()
            },
            ResponsePayload::TransactionOutputsWithProof(outputs_with_proof) => {
                outputs_with_proof.transactions_and_outputs.len()
            },
            _ => invalid_response_type!(client_response_payload),
        };
        if num_received_versions == 0 {
            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                "Received an empty response! Request: {:?}",
                self.request
            )));
        }

        // Identify the last received version and bound it appropriately
        let last_received_version = request_start_version
            .checked_add(num_received_versions as u64)
            .and_then(|version| version.checked_sub(1))
            .ok_or_else(|| Error::IntegerOverflow("Last received version has overflown!".into()))?;
        let last_received_version = bound_by_range(
            last_received_version,
            request_start_version,
            request_end_version,
        );

        // Update the local stream notification tracker
        self.next_stream_version = last_received_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next stream version has overflown!".into()))?;

        // Check if the stream is complete
        if last_received_version >= stream_end_version {
            self.stream_is_complete = true;
        }

        Ok(())
    }

    fn update_request_version(&mut self, request_end_version: Version) -> Result<(), Error> {
        self.next_request_version = request_end_version
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next request version has overflown!".into()))?;
        Ok(())
    }

    fn update_request_tracking(
        &mut self,
        client_requests: &[DataClientRequest],
    ) -> Result<(), Error> {
        match &self.request {
            StreamRequest::GetAllTransactions(_) => {
                for client_request in client_requests.iter() {
                    match client_request {
                        TransactionsWithProof(request) => {
                            self.update_request_version(request.end_version)?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            StreamRequest::GetAllTransactionOutputs(_) => {
                for client_request in client_requests.iter() {
                    match client_request {
                        TransactionOutputsWithProof(request) => {
                            self.update_request_version(request.end_version)?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            StreamRequest::GetAllTransactionsOrOutputs(_) => {
                for client_request in client_requests.iter() {
                    match client_request {
                        TransactionsOrOutputsWithProof(request) => {
                            self.update_request_version(request.end_version)?;
                        },
                        request => invalid_client_request!(request, self),
                    }
                }
            },
            request => invalid_stream_request!(request),
        }

        Ok(())
    }
}

impl DataStreamEngine for TransactionStreamEngine {
    fn create_data_client_requests(
        &mut self,
        max_number_of_requests: u64,
        max_in_flight_requests: u64,
        num_in_flight_requests: u64,
        global_data_summary: &GlobalDataSummary,
        _unique_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Vec<DataClientRequest>, Error> {
        // Calculate the request end version and optimal chunk sizes
        let (request_end_version, optimal_chunk_sizes) = match &self.request {
            StreamRequest::GetAllTransactions(request) => (
                request.end_version,
                global_data_summary
                    .optimal_chunk_sizes
                    .transaction_chunk_size,
            ),
            StreamRequest::GetAllTransactionOutputs(request) => (
                request.end_version,
                global_data_summary
                    .optimal_chunk_sizes
                    .transaction_output_chunk_size,
            ),
            StreamRequest::GetAllTransactionsOrOutputs(request) => (
                request.end_version,
                global_data_summary
                    .optimal_chunk_sizes
                    .transaction_output_chunk_size,
            ),
            request => invalid_stream_request!(request),
        };

        // Calculate the number of requests to send
        let num_requests_to_send = calculate_num_requests_to_send(
            max_number_of_requests,
            max_in_flight_requests,
            num_in_flight_requests,
        );

        // Create the client requests
        let client_requests = create_data_client_request_batch(
            self.next_request_version,
            request_end_version,
            num_requests_to_send,
            optimal_chunk_sizes,
            self.clone().into(),
        )?;

        // Return the requests
        self.update_request_tracking(&client_requests)?;
        Ok(client_requests)
    }

    fn is_remaining_data_available(&self, advertised_data: &AdvertisedData) -> Result<bool, Error> {
        let (request_end_version, advertised_ranges) = match &self.request {
            StreamRequest::GetAllTransactions(request) => {
                (request.end_version, &advertised_data.transactions)
            },
            StreamRequest::GetAllTransactionOutputs(request) => {
                (request.end_version, &advertised_data.transaction_outputs)
            },
            StreamRequest::GetAllTransactionsOrOutputs(request) => {
                (request.end_version, &advertised_data.transaction_outputs)
            },
            request => invalid_stream_request!(request),
        };
        Ok(AdvertisedData::contains_range(
            self.next_stream_version,
            request_end_version,
            advertised_ranges,
        ))
    }

    fn is_stream_complete(&self) -> bool {
        self.stream_is_complete
    }

    fn transform_client_response_into_notification(
        &mut self,
        client_request: &DataClientRequest,
        client_response_payload: ResponsePayload,
        notification_id_generator: Arc<U64IdGenerator>,
    ) -> Result<Option<DataNotification>, Error> {
        // Update the metrics for the number of received items
        update_response_chunk_size_metrics(client_request, &client_response_payload);

        // Identify the version information of the stream and client requests
        let (stream_end_version, request_start_version, request_end_version) = match &self.request {
            StreamRequest::GetAllTransactions(stream_request) => match client_request {
                TransactionsWithProof(request) => (
                    stream_request.end_version,
                    request.start_version,
                    request.end_version,
                ),
                request => invalid_client_request!(request, self),
            },
            StreamRequest::GetAllTransactionOutputs(stream_request) => match client_request {
                TransactionOutputsWithProof(request) => (
                    stream_request.end_version,
                    request.start_version,
                    request.end_version,
                ),
                request => invalid_client_request!(request, self),
            },
            StreamRequest::GetAllTransactionsOrOutputs(stream_request) => match client_request {
                TransactionsOrOutputsWithProof(request) => (
                    stream_request.end_version,
                    request.start_version,
                    request.end_version,
                ),
                request => invalid_client_request!(request, self),
            },
            request => invalid_stream_request!(request),
        };

        // Update the stream version
        self.update_stream_version(
            request_start_version,
            request_end_version,
            stream_end_version,
            &client_response_payload,
        )?;

        // Create a new data notification
        let data_notification = create_data_notification(
            notification_id_generator,
            client_response_payload,
            None,
            self.clone().into(),
        )?;
        Ok(Some(data_notification))
    }
}

/// A simple struct that tracks the state of a subscription stream.
#[derive(Clone, Debug)]
struct SubscriptionStream {
    known_version_at_stream_start: u64, // The highest known transaction version at stream start
    known_epoch_at_stream_start: u64,   // The highest known epoch at stream start
    subscription_stream_id: u64,        // The unique id of the subscription stream

    next_subscription_stream_index: u64, // The next request index to send for the stream
    max_subscription_stream_index: u64,  // The maximum request index to send for the stream
}

impl SubscriptionStream {
    pub fn new(
        data_streaming_config: DataStreamingServiceConfig,
        unique_id_generator: Arc<U64IdGenerator>,
        known_version_at_stream_start: u64,
        known_epoch_at_stream_start: u64,
    ) -> Self {
        // Generate a new subscription stream ID
        let subscription_stream_id = unique_id_generator.next();

        // Log the creation of the subscription stream
        debug!(
            (LogSchema::new(LogEntry::CreatedSubscriptionStream).message(&format!(
                "Created new subscription stream. Stream ID: {:?}",
                subscription_stream_id
            )))
        );

        // Calculate the maximum subscription stream index
        let max_subscription_stream_index = data_streaming_config
            .max_num_consecutive_subscriptions
            .saturating_sub(1);

        Self {
            known_version_at_stream_start,
            known_epoch_at_stream_start,
            subscription_stream_id,
            next_subscription_stream_index: 0,
            max_subscription_stream_index,
        }
    }

    /// Returns the known version and epoch at stream start
    pub fn get_known_version_and_epoch_at_stream_start(&self) -> (u64, u64) {
        (
            self.known_version_at_stream_start,
            self.known_epoch_at_stream_start,
        )
    }

    /// Returns the maximum subscription stream index
    pub fn get_max_subscription_stream_index(&self) -> u64 {
        self.max_subscription_stream_index
    }

    /// Returns the next subscription stream index
    pub fn get_next_subscription_stream_index(&self) -> u64 {
        self.next_subscription_stream_index
    }

    /// Returns the subscription stream ID
    pub fn get_subscription_stream_id(&self) -> u64 {
        self.subscription_stream_id
    }

    /// Increments the next subscription stream index
    pub fn increment_subscription_stream_index(&mut self) {
        self.next_subscription_stream_index += 1;
    }
}

/// Bounds the given number by the specified min and max values, inclusive.
/// If the number is less than the min, the min is returned. If the number is
/// greater than the max, the max is returned. Otherwise, the number is returned.
pub(crate) fn bound_by_range(number: u64, min: u64, max: u64) -> u64 {
    number.clamp(min, max)
}

/// Verifies that the `expected_next_index` matches the `start_index` and that
/// the `end_index` is greater than or equal to `expected_next_index`.
fn verify_client_request_indices(
    expected_next_index: u64,
    start_index: u64,
    end_index: u64,
) -> Result<(), Error> {
    if start_index != expected_next_index {
        return Err(Error::UnexpectedErrorEncountered(format!(
            "The start index did not match the expected next index! Given: {:?}, expected: {:?}",
            start_index, expected_next_index
        )));
    }

    if end_index < expected_next_index {
        return Err(Error::UnexpectedErrorEncountered(format!(
            "The end index was less than the expected next index! Given: {:?}, expected: {:?}",
            end_index, expected_next_index
        )));
    }

    Ok(())
}

/// Calculates the number of requests to send based
/// on the number of remaining in-flight request slots
/// and the maximum number of requests to send.
fn calculate_num_requests_to_send(
    max_number_of_requests: u64,
    max_in_flight_requests: u64,
    num_in_flight_requests: u64,
) -> u64 {
    // Calculate the number of remaining in-flight request slots
    let remaining_in_flight_slots = max_in_flight_requests.saturating_sub(num_in_flight_requests);

    // Bound the number of requests to send by the maximum
    min(remaining_in_flight_slots, max_number_of_requests)
}

/// Creates a batch of data client requests for the given stream engine
fn create_data_client_request_batch(
    start_index: u64,
    end_index: u64,
    max_number_of_requests: u64,
    optimal_chunk_size: u64,
    stream_engine: StreamEngine,
) -> Result<Vec<DataClientRequest>, Error> {
    if start_index > end_index {
        return Ok(vec![]);
    }

    // Calculate the total number of items left to satisfy the stream
    let mut total_items_to_fetch = end_index
        .checked_sub(start_index)
        .and_then(|e| e.checked_add(1)) // = end_index - start_index + 1
        .ok_or_else(|| Error::IntegerOverflow("Total items to fetch has overflown!".into()))?;

    // Iterate until we've requested all transactions or hit the maximum number of requests
    let mut data_client_requests = vec![];
    let mut num_requests_made = 0;
    let mut next_index_to_request = start_index;
    while total_items_to_fetch > 0 && num_requests_made < max_number_of_requests {
        // Calculate the number of items to fetch in this request
        let num_items_to_fetch = cmp::min(total_items_to_fetch, optimal_chunk_size);

        // Calculate the start and end indices for the request
        let request_start_index = next_index_to_request;
        let request_end_index = request_start_index
            .checked_add(num_items_to_fetch)
            .and_then(|e| e.checked_sub(1)) // = request_start_index + num_items_to_fetch - 1
            .ok_or_else(|| Error::IntegerOverflow("End index to fetch has overflown!".into()))?;

        // Create the data client requests
        let data_client_request =
            create_data_client_request(request_start_index, request_end_index, &stream_engine)?;
        data_client_requests.push(data_client_request);

        // Update the local loop state
        next_index_to_request = request_end_index
            .checked_add(1)
            .ok_or_else(|| Error::IntegerOverflow("Next index to request has overflown!".into()))?;
        total_items_to_fetch = total_items_to_fetch
            .checked_sub(num_items_to_fetch)
            .ok_or_else(|| Error::IntegerOverflow("Total items to fetch has overflown!".into()))?;
        num_requests_made = num_requests_made.checked_add(1).ok_or_else(|| {
            Error::IntegerOverflow("Number of payload requests has overflown!".into())
        })?;
    }

    Ok(data_client_requests)
}

/// Creates a data client request for the given stream engine using the
/// specified start and end indices.
fn create_data_client_request(
    start_index: u64,
    end_index: u64,
    stream_engine: &StreamEngine,
) -> Result<DataClientRequest, Error> {
    let data_client_request = match stream_engine {
        StreamEngine::StateStreamEngine(stream_engine) => {
            StateValuesWithProof(StateValuesWithProofRequest {
                version: stream_engine.request.version,
                start_index,
                end_index,
            })
        },
        StreamEngine::ContinuousTransactionStreamEngine(stream_engine) => {
            let target_ledger_info_version = stream_engine
                .get_target_ledger_info()?
                .ledger_info()
                .version();
            match &stream_engine.request {
                StreamRequest::ContinuouslyStreamTransactions(request) => {
                    TransactionsWithProof(TransactionsWithProofRequest {
                        start_version: start_index,
                        end_version: end_index,
                        proof_version: target_ledger_info_version,
                        include_events: request.include_events,
                    })
                },
                StreamRequest::ContinuouslyStreamTransactionOutputs(_) => {
                    TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                        start_version: start_index,
                        end_version: end_index,
                        proof_version: target_ledger_info_version,
                    })
                },
                StreamRequest::ContinuouslyStreamTransactionsOrOutputs(request) => {
                    TransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
                        start_version: start_index,
                        end_version: end_index,
                        proof_version: target_ledger_info_version,
                        include_events: request.include_events,
                    })
                },
                request => invalid_stream_request!(request),
            }
        },
        StreamEngine::EpochEndingStreamEngine(_) => {
            EpochEndingLedgerInfos(EpochEndingLedgerInfosRequest {
                start_epoch: start_index,
                end_epoch: end_index,
            })
        },
        StreamEngine::TransactionStreamEngine(stream_engine) => match &stream_engine.request {
            StreamRequest::GetAllTransactions(request) => {
                TransactionsWithProof(TransactionsWithProofRequest {
                    start_version: start_index,
                    end_version: end_index,
                    proof_version: request.proof_version,
                    include_events: request.include_events,
                })
            },
            StreamRequest::GetAllTransactionOutputs(request) => {
                TransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                    start_version: start_index,
                    end_version: end_index,
                    proof_version: request.proof_version,
                })
            },
            StreamRequest::GetAllTransactionsOrOutputs(request) => {
                TransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
                    start_version: start_index,
                    end_version: end_index,
                    proof_version: request.proof_version,
                    include_events: request.include_events,
                })
            },
            request => invalid_stream_request!(request),
        },
    };
    Ok(data_client_request)
}

/// Creates a new data notification for the given client response.
fn create_data_notification(
    notification_id_generator: Arc<U64IdGenerator>,
    client_response: ResponsePayload,
    target_ledger_info: Option<LedgerInfoWithSignatures>,
    stream_engine: StreamEngine,
) -> Result<DataNotification, Error> {
    // Create a unique notification ID
    let notification_id = notification_id_generator.next();

    // Get the data payload
    let client_response_type = client_response.get_label();
    let data_payload = match client_response {
        ResponsePayload::StateValuesWithProof(states_chunk) => {
            DataPayload::StateValuesWithProof(states_chunk)
        },
        ResponsePayload::EpochEndingLedgerInfos(ledger_infos) => {
            DataPayload::EpochEndingLedgerInfos(ledger_infos)
        },
        ResponsePayload::NewTransactionsWithProof((transactions_chunk, target_ledger_info)) => {
            match stream_engine {
                StreamEngine::ContinuousTransactionStreamEngine(_) => {
                    DataPayload::ContinuousTransactionsWithProof(
                        target_ledger_info,
                        transactions_chunk,
                    )
                },
                _ => invalid_response_type!(client_response_type),
            }
        },
        ResponsePayload::NewTransactionOutputsWithProof((
            transactions_output_chunk,
            target_ledger_info,
        )) => match stream_engine {
            StreamEngine::ContinuousTransactionStreamEngine(_) => {
                DataPayload::ContinuousTransactionOutputsWithProof(
                    target_ledger_info,
                    transactions_output_chunk.clone(),
                )
            },
            _ => invalid_response_type!(client_response_type),
        },
        ResponsePayload::TransactionsWithProof(transactions_chunk) => match stream_engine {
            StreamEngine::ContinuousTransactionStreamEngine(_) => {
                let target_ledger_info = target_ledger_info.ok_or_else(|| {
                    Error::UnexpectedErrorEncountered(
                        "The target ledger info was not provided".into(),
                    )
                })?;
                DataPayload::ContinuousTransactionsWithProof(target_ledger_info, transactions_chunk)
            },
            StreamEngine::TransactionStreamEngine(_) => {
                DataPayload::TransactionsWithProof(transactions_chunk)
            },
            _ => invalid_response_type!(client_response_type),
        },
        ResponsePayload::TransactionOutputsWithProof(transactions_output_chunk) => {
            match stream_engine {
                StreamEngine::ContinuousTransactionStreamEngine(_) => {
                    let target_ledger_info = target_ledger_info.ok_or_else(|| {
                        Error::UnexpectedErrorEncountered(
                            "The target ledger info was not provided".into(),
                        )
                    })?;
                    DataPayload::ContinuousTransactionOutputsWithProof(
                        target_ledger_info,
                        transactions_output_chunk,
                    )
                },
                StreamEngine::TransactionStreamEngine(_) => {
                    DataPayload::TransactionOutputsWithProof(transactions_output_chunk)
                },
                _ => invalid_response_type!(client_response_type),
            }
        },
        _ => invalid_response_type!(client_response_type),
    };

    // Create and return the data notification
    let data_notification = DataNotification::new(notification_id, data_payload);
    Ok(data_notification)
}

/// Extracts the number of new versions and target
/// ledger info for the given client response payload.
fn extract_new_versions_and_target(
    client_response_payload: &ResponsePayload,
) -> Result<(usize, LedgerInfoWithSignatures), Error> {
    // Extract the number of new versions and the target ledger info
    let (num_versions, target_ledger_info) = match &client_response_payload {
        ResponsePayload::NewTransactionsWithProof((
            transactions_with_proof,
            target_ledger_info,
        )) => (
            transactions_with_proof.transactions.len(),
            target_ledger_info.clone(),
        ),
        ResponsePayload::NewTransactionOutputsWithProof((
            outputs_with_proof,
            target_ledger_info,
        )) => (
            outputs_with_proof.transactions_and_outputs.len(),
            target_ledger_info.clone(),
        ),
        response_payload => {
            // TODO(joshlind): eventually we want to notify the data client of the bad response
            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                "Expected new transactions or outputs but got: {:?}",
                response_payload
            )));
        },
    };

    // Ensure that we have at least one data item
    if num_versions == 0 {
        // TODO(joshlind): eventually we want to notify the data client of the bad response
        return Err(Error::AptosDataClientResponseIsInvalid(
            "Received an empty transaction or output list!".into(),
        ));
    }

    Ok((num_versions, target_ledger_info))
}

/// Updates the response chunk size metrics for the given request and response
fn update_response_chunk_size_metrics(
    client_request: &DataClientRequest,
    client_response_payload: &ResponsePayload,
) {
    metrics::observe_values(
        &metrics::RECEIVED_DATA_RESPONSE_CHUNK_SIZE,
        client_request.get_label(),
        client_response_payload.get_label(),
        client_response_payload.get_data_chunk_size() as u64,
    );
}

/// Updates the metrics with a terminated subscription event and reason
fn update_terminated_subscription_metrics(termination_reason: &str) {
    metrics::increment_counter(&metrics::TERMINATE_SUBSCRIPTION_STREAM, termination_reason);
}
