// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_notification,
    data_notification::{
        DataClientRequest, DataNotification, DataPayload, EpochEndingLedgerInfosRequest,
        NewTransactionOutputsWithProofRequest, NewTransactionsOrOutputsWithProofRequest,
        NewTransactionsWithProofRequest, NotificationId, NumberOfStatesRequest,
        StateValuesWithProofRequest, SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    dynamic_prefetching::DynamicPrefetchingState,
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    metrics::{increment_counter, increment_counter_multiple_labels, start_timer},
    stream_engine::{DataStreamEngine, StreamEngine},
    streaming_client::{NotificationFeedback, StreamRequest},
    streaming_service::StreamUpdateNotification,
};
use aptos_channels::aptos_channel;
use aptos_config::config::{AptosDataClientConfig, DataStreamingServiceConfig};
use aptos_data_client::{
    global_summary::{AdvertisedData, GlobalDataSummary},
    interface::{
        AptosDataClientInterface, Response, ResponseContext, ResponseError, ResponsePayload,
        SubscriptionRequestMetadata,
    },
};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_time_service::{TimeService, TimeServiceTrait};
use futures::{channel::mpsc, stream::FusedStream, SinkExt, Stream};
use std::{
    cmp::min,
    collections::{BTreeMap, VecDeque},
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::task::JoinHandle;

// The frequency at which to log sent data request messages
const SENT_REQUESTS_LOG_FREQ_SECS: u64 = 1;

/// A unique ID used to identify each stream.
pub type DataStreamId = u64;

/// A pointer to a thread-safe `PendingClientResponse`.
pub type PendingClientResponse = Arc<Mutex<Box<data_notification::PendingClientResponse>>>;

/// Each data stream holds the original stream request from the client and tracks
/// the progress of the data stream to satisfy that request (e.g., the data that
/// has already been sent along the stream to the client and the in-flight Aptos
/// data client requests that have been sent to the network).
///
/// Note that it is the responsibility of the data stream to send data
/// notifications along the stream in sequential order (e.g., transactions and
/// proofs must be sent with monotonically increasing versions).
#[derive(Debug)]
pub struct DataStream<T> {
    // The configuration for the data client
    data_client_config: AptosDataClientConfig,

    // The configuration for the streaming service
    streaming_service_config: DataStreamingServiceConfig,

    // The unique ID for this data stream. This is useful for logging.
    data_stream_id: DataStreamId,

    // The data client through which to fetch data from the Aptos network
    aptos_data_client: T,

    // The engine for this data stream
    stream_engine: StreamEngine,

    // The stream update notifier (to notify the streaming service that
    // the stream has been updated, e.g., data is now ready to be processed).
    stream_update_notifier: aptos_channel::Sender<(), StreamUpdateNotification>,

    // The current queue of data client requests and pending responses. When the
    // request at the head of the queue completes (i.e., we receive a response),
    // a data notification can be created and sent along the stream.
    sent_data_requests: Option<VecDeque<PendingClientResponse>>,

    // Handles of all spawned tasks. This is useful for aborting the tasks in
    // the case the stream is terminated prematurely.
    spawned_tasks: Vec<JoinHandle<()>>,

    // Maps a notification ID (sent along the data stream) to a response context.
    notifications_to_responses: BTreeMap<NotificationId, ResponseContext>,

    // The channel on which to send data notifications when they are ready.
    notification_sender: mpsc::Sender<DataNotification>,

    // A unique notification ID generator
    notification_id_generator: Arc<U64IdGenerator>,

    // Notification ID of the end of stream notification (when it has been sent)
    stream_end_notification_id: Option<NotificationId>,

    // The current failure count of the request at the head of the request queue.
    // If this count becomes too large, the stream is evidently blocked (i.e.,
    // unable to make progress) and will automatically terminate.
    request_failure_count: u64,

    // Whether the data stream has encountered an error trying to send a
    // notification to the listener. If so, the stream is dead and it will
    // stop sending notifications. This handles when clients drop the listener.
    send_failure: bool,

    // The measured subscription stream lag (if any)
    subscription_stream_lag: Option<SubscriptionStreamLag>,

    // The time service to track elapsed time (e.g., during stream lag checks)
    time_service: TimeService,

    // The dynamic prefetching state (if enabled)
    dynamic_prefetching_state: DynamicPrefetchingState,
}

impl<T: AptosDataClientInterface + Send + Clone + 'static> DataStream<T> {
    pub fn new(
        data_client_config: AptosDataClientConfig,
        data_stream_config: DataStreamingServiceConfig,
        data_stream_id: DataStreamId,
        stream_request: &StreamRequest,
        stream_update_notifier: aptos_channel::Sender<(), StreamUpdateNotification>,
        aptos_data_client: T,
        notification_id_generator: Arc<U64IdGenerator>,
        advertised_data: &AdvertisedData,
        time_service: TimeService,
    ) -> Result<(Self, DataStreamListener), Error> {
        // Create a new data stream listener
        let (notification_sender, notification_receiver) =
            mpsc::channel(data_stream_config.max_data_stream_channel_sizes as usize);
        let data_stream_listener = DataStreamListener::new(data_stream_id, notification_receiver);

        // Create a new stream engine
        let stream_engine = StreamEngine::new(data_stream_config, stream_request, advertised_data)?;

        // Create the dynamic prefetching state
        let dynamic_prefetching_state =
            DynamicPrefetchingState::new(data_stream_config, time_service.clone());

        // Create a new data stream
        let data_stream = Self {
            data_client_config,
            streaming_service_config: data_stream_config,
            data_stream_id,
            aptos_data_client,
            stream_engine,
            stream_update_notifier,
            sent_data_requests: None,
            spawned_tasks: vec![],
            notifications_to_responses: BTreeMap::new(),
            notification_sender,
            notification_id_generator,
            stream_end_notification_id: None,
            request_failure_count: 0,
            send_failure: false,
            subscription_stream_lag: None,
            time_service,
            dynamic_prefetching_state,
        };

        Ok((data_stream, data_stream_listener))
    }

    /// Clears the sent data requests queue and drops all tasks
    pub fn clear_sent_data_requests_queue(&mut self) {
        // Clear all pending data requests
        if let Some(sent_data_requests) = self.sent_data_requests.as_mut() {
            sent_data_requests.clear();
        }

        // Abort all spawned tasks
        self.abort_spawned_tasks();
    }

    /// Returns true iff the first batch of data client requests has been sent
    pub fn data_requests_initialized(&self) -> bool {
        self.sent_data_requests.is_some()
    }

    /// Resets the subscription stream lag on the data stream
    fn reset_subscription_stream_lag(&mut self) {
        // Reset the subscription stream lag metrics
        metrics::set_subscription_stream_lag(0);

        // Reset the stream lag
        self.subscription_stream_lag = None;
    }

    /// Sets the subscription stream lag on the data stream
    fn set_subscription_stream_lag(&mut self, subscription_stream_lag: SubscriptionStreamLag) {
        // Update the subscription stream lag metrics
        metrics::set_subscription_stream_lag(subscription_stream_lag.version_lag);

        // Set the stream lag
        self.subscription_stream_lag = Some(subscription_stream_lag)
    }

    /// Initializes the data client requests by sending out the first batch
    pub fn initialize_data_requests(
        &mut self,
        global_data_summary: GlobalDataSummary,
    ) -> Result<(), Error> {
        // Initialize the data client requests queue
        self.sent_data_requests = Some(VecDeque::new());

        // Create and send the data client requests to the network
        self.create_and_send_client_requests(&global_data_summary)
    }

    /// Returns true iff the given `notification_id` was sent by this stream
    pub fn sent_notification(&self, notification_id: &NotificationId) -> bool {
        if let Some(stream_end_notification_id) = self.stream_end_notification_id {
            if stream_end_notification_id == *notification_id {
                return true;
            }
        }

        self.notifications_to_responses
            .contains_key(notification_id)
    }

    /// Notifies the Aptos data client of a bad client response
    pub fn handle_notification_feedback(
        &self,
        notification_id: &NotificationId,
        notification_feedback: &NotificationFeedback,
    ) -> Result<(), Error> {
        if self.stream_end_notification_id == Some(*notification_id) {
            return if matches!(notification_feedback, NotificationFeedback::EndOfStream) {
                Ok(())
            } else {
                Err(Error::UnexpectedErrorEncountered(format!(
                    "Invalid feedback given for stream end: {:?}",
                    notification_feedback
                )))
            };
        }

        let response_context = self
            .notifications_to_responses
            .get(notification_id)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered(format!(
                    "Response context missing for notification ID: {:?}",
                    notification_id
                ))
            })?;
        let response_error = extract_response_error(notification_feedback)?;
        self.notify_bad_response(response_context, response_error);

        Ok(())
    }

    /// Creates and sends a batch of data client requests to the network
    fn create_and_send_client_requests(
        &mut self,
        global_data_summary: &GlobalDataSummary,
    ) -> Result<(), Error> {
        // Calculate the number of in-flight requests (i.e., requests that haven't completed)
        let num_pending_requests = self.get_num_pending_data_requests()?;
        let num_complete_pending_requests = self.get_num_complete_pending_requests()?;
        let num_in_flight_requests =
            num_pending_requests.saturating_sub(num_complete_pending_requests);

        // Calculate the max number of requests that can be sent now
        let max_pending_requests = self.streaming_service_config.max_pending_requests;
        let max_num_requests_to_send = max_pending_requests.saturating_sub(num_pending_requests);

        // Send the client requests iff we have enough room in the queue
        if max_num_requests_to_send > 0 {
            // Get the max number of in-flight requests from the prefetching state
            let max_in_flight_requests = self
                .dynamic_prefetching_state
                .get_max_concurrent_requests(&self.stream_engine);

            // Create the client requests
            let client_requests = self.stream_engine.create_data_client_requests(
                max_num_requests_to_send,
                max_in_flight_requests,
                num_in_flight_requests,
                global_data_summary,
                self.notification_id_generator.clone(),
            )?;

            // Add the client requests to the sent data requests queue
            for client_request in &client_requests {
                // Send the client request
                let pending_client_response =
                    self.send_client_request(false, client_request.clone());

                // Enqueue the pending response
                self.get_sent_data_requests()?
                    .push_back(pending_client_response);
            }

            // Log the number of sent data requests
            sample!(
                SampleRate::Duration(Duration::from_secs(SENT_REQUESTS_LOG_FREQ_SECS)),
                debug!(
                    (LogSchema::new(LogEntry::SendDataRequests)
                        .stream_id(self.data_stream_id)
                        .event(LogEvent::Success)
                        .message(&format!(
                            "Sent {:?} data requests to the network",
                            client_requests.len()
                        )))
                )
            );
        }

        // Update the counters for the complete and pending responses
        metrics::set_complete_pending_data_responses(num_complete_pending_requests);
        metrics::set_pending_data_responses(self.get_num_pending_data_requests()?);

        Ok(())
    }

    /// Sends a given request to the data client to be forwarded to the network
    /// and returns a pending client response. If `request_retry` is true
    /// exponential backoff takes affect (i.e., to increase the request timeout).
    fn send_client_request(
        &mut self,
        request_retry: bool,
        data_client_request: DataClientRequest,
    ) -> PendingClientResponse {
        // Create a new pending client response
        let pending_client_response = Arc::new(Mutex::new(Box::new(
            data_notification::PendingClientResponse::new(data_client_request.clone()),
        )));

        // Calculate the request timeout to use, based on the
        // request type and the number of previous failures.
        let request_timeout_ms = if data_client_request.is_optimistic_fetch_request() {
            self.data_client_config.optimistic_fetch_timeout_ms
        } else if data_client_request.is_subscription_request() {
            self.data_client_config.subscription_response_timeout_ms
        } else if !request_retry {
            self.data_client_config.response_timeout_ms
        } else {
            let response_timeout_ms = self.data_client_config.response_timeout_ms;
            let max_response_timeout_ms = self.data_client_config.max_response_timeout_ms;

            // Exponentially increase the timeout based on the number of
            // previous failures (but bounded by the max timeout).
            let request_timeout_ms = min(
                max_response_timeout_ms,
                response_timeout_ms * (u32::pow(2, self.request_failure_count as u32) as u64),
            );

            // Update the retry counter and log the request
            increment_counter_multiple_labels(
                &metrics::RETRIED_DATA_REQUESTS,
                data_client_request.get_label(),
                &request_timeout_ms.to_string(),
            );
            info!(
                (LogSchema::new(LogEntry::RetryDataRequest)
                    .stream_id(self.data_stream_id)
                    .message(&format!(
                        "Retrying data request type: {:?}, with new timeout: {:?} (ms)",
                        data_client_request.get_label(),
                        request_timeout_ms.to_string()
                    )))
            );

            request_timeout_ms
        };

        // Send the request to the network
        let join_handle = spawn_request_task(
            self.data_stream_id,
            data_client_request,
            self.aptos_data_client.clone(),
            pending_client_response.clone(),
            request_timeout_ms,
            self.stream_update_notifier.clone(),
        );
        self.spawned_tasks.push(join_handle);

        pending_client_response
    }

    // TODO(joshlind): this function shouldn't be blocking when trying to send.
    // If there are multiple streams, a single blocked stream could cause them
    // all to block. This is acceptable for now (because there is only ever
    // a single stream in use by the driver) but it should be fixed if we want
    // to generalize this for multiple streams.
    async fn send_data_notification(
        &mut self,
        data_notification: DataNotification,
    ) -> Result<(), Error> {
        if let Err(error) = self.notification_sender.send(data_notification).await {
            let error = Error::UnexpectedErrorEncountered(error.to_string());
            warn!(
                (LogSchema::new(LogEntry::StreamNotification)
                    .stream_id(self.data_stream_id)
                    .event(LogEvent::Error)
                    .error(&error)
                    .message("Failed to send data notification to listener!"))
            );
            self.send_failure = true;
            Err(error)
        } else {
            Ok(())
        }
    }

    /// Returns true iff there was a send failure
    pub fn send_failure(&self) -> bool {
        self.send_failure
    }

    async fn send_end_of_stream_notification(&mut self) -> Result<(), Error> {
        // Create end of stream notification
        let notification_id = self.notification_id_generator.next();
        let data_notification = DataNotification::new(notification_id, DataPayload::EndOfStream);

        // Send the data notification
        info!(
            (LogSchema::new(LogEntry::EndOfStreamNotification)
                .stream_id(self.data_stream_id)
                .event(LogEvent::Pending)
                .message("Sent the end of stream notification"))
        );
        self.stream_end_notification_id = Some(notification_id);
        self.send_data_notification(data_notification).await
    }

    /// Processes any data client responses that have been received. Note: the
    /// responses must be processed in FIFO order.
    pub async fn process_data_responses(
        &mut self,
        global_data_summary: GlobalDataSummary,
    ) -> Result<(), Error> {
        if self.stream_engine.is_stream_complete()
            || self.request_failure_count >= self.streaming_service_config.max_request_retry
            || self.send_failure
        {
            if !self.send_failure && self.stream_end_notification_id.is_none() {
                self.send_end_of_stream_notification().await?;
            }
            return Ok(()); // There's nothing left to do
        }

        // Continuously process any ready data responses
        while let Some(pending_response) = self.pop_pending_response_queue()? {
            // Get the client request and response information
            let maybe_client_response = pending_response.lock().client_response.take();
            let client_response = maybe_client_response.ok_or_else(|| {
                Error::UnexpectedErrorEncountered("The client response should be ready!".into())
            })?;
            let client_request = &pending_response.lock().client_request.clone();

            // Process the client response
            match client_response {
                Ok(client_response) => {
                    // Sanity check and process the response
                    if sanity_check_client_response_type(client_request, &client_response) {
                        // If the response wasn't enough to satisfy the original request (e.g.,
                        // it was truncated), missing data should be requested.
                        let mut head_of_line_blocked = false;
                        match self.request_missing_data(client_request, &client_response.payload) {
                            Ok(missing_data_requested) => {
                                if missing_data_requested {
                                    head_of_line_blocked = true; // We're now head of line blocked on the missing data
                                }
                            },
                            Err(error) => {
                                warn!(LogSchema::new(LogEntry::ReceivedDataResponse)
                                    .stream_id(self.data_stream_id)
                                    .event(LogEvent::Error)
                                    .error(&error)
                                    .message("Failed to determine if missing data was requested!"));
                            },
                        }

                        // If the request was a subscription request and the subscription
                        // stream is lagging behind the data advertisements, the stream
                        // engine should be notified (e.g., so that it can catch up).
                        if client_request.is_subscription_request() {
                            if let Err(error) = self.check_subscription_stream_lag(
                                &global_data_summary,
                                &client_response.payload,
                            ) {
                                self.notify_new_data_request_error(client_request, error)?;
                                head_of_line_blocked = true; // We're now head of line blocked on the failed stream
                            }
                        }

                        // The response is valid, send the data notification to the client
                        self.send_data_notification_to_client(client_request, client_response)
                            .await?;

                        // If the request is for specific data, increase the prefetching limit.
                        // Note: we don't increase the limit for new data requests because
                        // those don't invoke the prefetcher (as we're already up-to-date).
                        if !client_request.is_new_data_request() {
                            self.dynamic_prefetching_state
                                .increase_max_concurrent_requests();
                        }

                        // If we're head of line blocked, we should return early
                        if head_of_line_blocked {
                            break;
                        }
                    } else {
                        // The sanity check failed
                        self.handle_sanity_check_failure(client_request, &client_response.context)?;
                        break; // We're now head of line blocked on the failed request
                    }
                },
                Err(error) => {
                    // Handle the error depending on the request type
                    if client_request.is_new_data_request() {
                        // The request was for new data. We should notify the
                        // stream engine and clear the requests queue.
                        self.notify_new_data_request_error(client_request, error)?;
                    } else {
                        // Decrease the prefetching limit on an error
                        self.dynamic_prefetching_state
                            .decrease_max_concurrent_requests();

                        // Handle the error and simply retry
                        self.handle_data_client_error(client_request, &error)?;
                    }
                    break; // We're now head of line blocked on the failed request
                },
            }
        }

        // Create and send further client requests to the network
        // to ensure we're maximizing the number of concurrent requests.
        self.create_and_send_client_requests(&global_data_summary)
    }

    /// Verifies that the subscription stream is not lagging too much (i.e.,
    /// behind the data advertisements). If it is, an error is returned.
    fn check_subscription_stream_lag(
        &mut self,
        global_data_summary: &GlobalDataSummary,
        response_payload: &ResponsePayload,
    ) -> Result<(), aptos_data_client::error::Error> {
        // Get the highest version sent in the subscription response
        let highest_response_version = match response_payload {
            ResponsePayload::NewTransactionsWithProof((transactions_with_proof, _)) => {
                if let Some(first_version) = transactions_with_proof.get_first_transaction_version()
                {
                    let num_transactions = transactions_with_proof.get_num_transactions();
                    first_version
                        .saturating_add(num_transactions as u64)
                        .saturating_sub(1) // first_version + num_txns - 1
                } else {
                    return Err(aptos_data_client::error::Error::UnexpectedErrorEncountered(
                        "The first transaction version is missing from the stream response!".into(),
                    ));
                }
            },
            ResponsePayload::NewTransactionOutputsWithProof((outputs_with_proof, _)) => {
                if let Some(first_version) = outputs_with_proof.get_first_output_version() {
                    let num_outputs = outputs_with_proof.get_num_outputs();
                    first_version
                        .saturating_add(num_outputs as u64)
                        .saturating_sub(1) // first_version + num_outputs - 1
                } else {
                    return Err(aptos_data_client::error::Error::UnexpectedErrorEncountered(
                        "The first output version is missing from the stream response!".into(),
                    ));
                }
            },
            _ => {
                return Ok(()); // The response payload doesn't contain a subscription response
            },
        };

        // Get the highest advertised version
        let highest_advertised_version = global_data_summary
            .advertised_data
            .highest_synced_ledger_info()
            .map(|ledger_info| ledger_info.ledger_info().version())
            .ok_or_else(|| {
                aptos_data_client::error::Error::UnexpectedErrorEncountered(
                    "The highest synced ledger info is missing from the global data summary!"
                        .into(),
                )
            })?;

        // If the stream is not lagging behind, reset the lag and return
        if highest_response_version >= highest_advertised_version {
            self.reset_subscription_stream_lag();
            return Ok(());
        }

        // Otherwise, the stream is lagging behind the advertised version.
        // Check if the stream is beyond recovery (i.e., has failed).
        let current_stream_lag =
            highest_advertised_version.saturating_sub(highest_response_version);
        if let Some(mut subscription_stream_lag) = self.subscription_stream_lag.take() {
            // Check if the stream lag is beyond recovery
            if subscription_stream_lag
                .is_beyond_recovery(self.streaming_service_config, current_stream_lag)
            {
                return Err(
                    aptos_data_client::error::Error::SubscriptionStreamIsLagging(format!(
                        "The subscription stream is beyond recovery! Current lag: {:?}, last lag: {:?},",
                        current_stream_lag, subscription_stream_lag.version_lag
                    )),
                );
            }

            // The stream is lagging, but it's not yet beyond recovery
            self.set_subscription_stream_lag(subscription_stream_lag);
        } else {
            // The stream was not previously lagging, but it is now!
            let subscription_stream_lag =
                SubscriptionStreamLag::new(current_stream_lag, self.time_service.clone());
            self.set_subscription_stream_lag(subscription_stream_lag);
        }

        Ok(())
    }

    /// Notifies the stream engine that a new data request error was encountered
    fn notify_new_data_request_error(
        &mut self,
        client_request: &DataClientRequest,
        error: aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        // Notify the stream engine and clear the requests queue
        self.stream_engine
            .notify_new_data_request_error(client_request, error)?;
        self.clear_sent_data_requests_queue();

        Ok(())
    }

    /// Requests any missing data from the previous client response
    /// and returns true iff missing data was requested.
    fn request_missing_data(
        &mut self,
        data_client_request: &DataClientRequest,
        response_payload: &ResponsePayload,
    ) -> Result<bool, Error> {
        // Identify if any missing data needs to be requested
        if let Some(missing_data_request) =
            create_missing_data_request(data_client_request, response_payload)?
        {
            // Increment the missing client request counter
            increment_counter(
                &metrics::SENT_DATA_REQUESTS_FOR_MISSING_DATA,
                data_client_request.get_label(),
            );

            // Send the missing data request
            let pending_client_response =
                self.send_client_request(false, missing_data_request.clone());

            // Push the pending response to the front of the queue
            self.get_sent_data_requests()?
                .push_front(pending_client_response);

            return Ok(true); // Missing data was requested
        }

        Ok(false) // No missing data was requested
    }

    /// Pops and returns the first pending client response if the response has
    /// been received. Returns `None` otherwise.
    fn pop_pending_response_queue(&mut self) -> Result<Option<PendingClientResponse>, Error> {
        let sent_data_requests = self.get_sent_data_requests()?;
        let pending_client_response = if let Some(data_request) = sent_data_requests.front() {
            if data_request.lock().client_response.is_some() {
                // We've received a response! Pop the requests off the queue.
                sent_data_requests.pop_front()
            } else {
                None
            }
        } else {
            None
        };
        Ok(pending_client_response)
    }

    /// Handles a client response that failed sanity checks
    fn handle_sanity_check_failure(
        &mut self,
        data_client_request: &DataClientRequest,
        response_context: &ResponseContext,
    ) -> Result<(), Error> {
        error!(LogSchema::new(LogEntry::ReceivedDataResponse)
            .stream_id(self.data_stream_id)
            .event(LogEvent::Error)
            .message("Encountered a client response that failed the sanity checks!"));

        self.notify_bad_response(response_context, ResponseError::InvalidPayloadDataType);
        self.resend_data_client_request(data_client_request)
    }

    /// Handles an error returned by the data client in relation to a request
    fn handle_data_client_error(
        &mut self,
        data_client_request: &DataClientRequest,
        data_client_error: &aptos_data_client::error::Error,
    ) -> Result<(), Error> {
        // Log the error
        warn!(LogSchema::new(LogEntry::ReceivedDataResponse)
            .stream_id(self.data_stream_id)
            .event(LogEvent::Error)
            .error(&data_client_error.clone().into())
            .message("Encountered a data client error!"));

        // TODO(joshlind): can we identify the best way to react to the error?
        self.resend_data_client_request(data_client_request)
    }

    /// Resends a failed data client request and pushes the pending notification
    /// to the head of the pending notifications batch.
    fn resend_data_client_request(
        &mut self,
        data_client_request: &DataClientRequest,
    ) -> Result<(), Error> {
        // Increment the number of client failures for this request
        self.request_failure_count += 1;

        // Resend the client request
        let pending_client_response = self.send_client_request(true, data_client_request.clone());

        // Push the pending response to the head of the sent requests queue
        self.get_sent_data_requests()?
            .push_front(pending_client_response);

        Ok(())
    }

    /// Notifies the Aptos data client of a bad client response
    fn notify_bad_response(
        &self,
        response_context: &ResponseContext,
        response_error: ResponseError,
    ) {
        let response_id = response_context.id;
        info!(LogSchema::new(LogEntry::ReceivedDataResponse)
            .stream_id(self.data_stream_id)
            .event(LogEvent::Error)
            .message(&format!(
                "Notifying the data client of a bad response. Response id: {:?}, error: {:?}",
                response_id, response_error
            )));

        response_context
            .response_callback
            .notify_bad_response(response_error);
    }

    /// Sends a data notification to the client along the stream
    async fn send_data_notification_to_client(
        &mut self,
        data_client_request: &DataClientRequest,
        data_client_response: Response<ResponsePayload>,
    ) -> Result<(), Error> {
        let (response_context, response_payload) = data_client_response.into_parts();

        // Create a new data notification
        if let Some(data_notification) = self
            .stream_engine
            .transform_client_response_into_notification(
                data_client_request,
                response_payload,
                self.notification_id_generator.clone(),
            )?
        {
            // Update the metrics for the data notification send latency
            metrics::observe_duration(
                &metrics::DATA_NOTIFICATION_SEND_LATENCY,
                data_client_request.get_label(),
                response_context.creation_time,
            );

            // Save the response context for this notification ID
            let notification_id = data_notification.notification_id;
            self.insert_notification_response_mapping(notification_id, response_context)?;

            // Send the notification along the stream
            trace!(
                (LogSchema::new(LogEntry::StreamNotification)
                    .stream_id(self.data_stream_id)
                    .event(LogEvent::Success)
                    .message(&format!(
                        "Sent a single stream notification! Notification ID: {:?}",
                        notification_id
                    )))
            );
            self.send_data_notification(data_notification).await?;

            // Reset the failure count. We've sent a notification and can move on.
            self.request_failure_count = 0;
        }

        Ok(())
    }

    fn insert_notification_response_mapping(
        &mut self,
        notification_id: NotificationId,
        response_context: ResponseContext,
    ) -> Result<(), Error> {
        if let Some(response_context) = self
            .notifications_to_responses
            .insert(notification_id, response_context)
        {
            Err(Error::UnexpectedErrorEncountered(format!(
                "Duplicate sent notification ID found! \
                 Notification ID: {:?}, \
                 previous Response context: {:?}",
                notification_id, response_context
            )))
        } else {
            self.garbage_collect_notification_response_map()
        }
    }

    fn garbage_collect_notification_response_map(&mut self) -> Result<(), Error> {
        let max_notification_id_mappings =
            self.streaming_service_config.max_notification_id_mappings;
        let map_length = self.notifications_to_responses.len() as u64;
        if map_length > max_notification_id_mappings {
            let num_entries_to_remove = map_length
                .checked_sub(max_notification_id_mappings)
                .ok_or_else(|| {
                    Error::IntegerOverflow("Number of entries to remove has overflown!".into())
                })?;

            // Collect all the keys that need to removed. Note: BTreeMap keys
            // are sorted, so we'll remove the lowest notification IDs. These
            // will be the oldest notifications.
            let mut all_keys = self.notifications_to_responses.keys();
            let mut keys_to_remove = vec![];
            for _ in 0..num_entries_to_remove {
                if let Some(key_to_remove) = all_keys.next() {
                    keys_to_remove.push(*key_to_remove);
                }
            }

            // Remove the keys
            for key_to_remove in &keys_to_remove {
                self.notifications_to_responses.remove(key_to_remove);
            }
        }

        Ok(())
    }

    /// Verifies that the data required by the stream can be satisfied using the
    /// currently advertised data in the network. If not, returns an error.
    pub fn ensure_data_is_available(&self, advertised_data: &AdvertisedData) -> Result<(), Error> {
        if !self
            .stream_engine
            .is_remaining_data_available(advertised_data)?
        {
            return Err(Error::DataIsUnavailable(format!(
                "Unable to satisfy stream engine: {:?}, with advertised data: {:?}",
                self.stream_engine, advertised_data
            )));
        }
        Ok(())
    }

    /// Returns the number of pending requests in the sent data requests queue
    /// that have already completed (i.e., are no longer in-flight).
    fn get_num_complete_pending_requests(&mut self) -> Result<u64, Error> {
        let mut num_complete_pending_requests = 0;
        for sent_data_request in self.get_sent_data_requests()? {
            if let Some(client_response) = sent_data_request.lock().client_response.as_ref() {
                if client_response.is_ok() {
                    // Only count successful responses as complete. Failures will be retried
                    num_complete_pending_requests += 1;
                }
            }
        }
        Ok(num_complete_pending_requests)
    }

    /// Returns the number of pending requests in the sent data requests queue
    fn get_num_pending_data_requests(&mut self) -> Result<u64, Error> {
        let pending_data_requests = self.get_sent_data_requests()?;
        let num_pending_data_requests = pending_data_requests.len() as u64;
        Ok(num_pending_data_requests)
    }

    /// Assumes the caller has already verified that `sent_data_requests` has
    /// been initialized.
    fn get_sent_data_requests(&mut self) -> Result<&mut VecDeque<PendingClientResponse>, Error> {
        self.sent_data_requests.as_mut().ok_or_else(|| {
            Error::UnexpectedErrorEncountered("Sent data requests should be initialized!".into())
        })
    }

    #[cfg(test)]
    /// This is exposed and used only for test purposes.
    pub fn get_sent_requests_and_notifications(
        &mut self,
    ) -> (
        &mut Option<VecDeque<PendingClientResponse>>,
        &mut BTreeMap<NotificationId, ResponseContext>,
    ) {
        let sent_requests = &mut self.sent_data_requests;
        let sent_notifications = &mut self.notifications_to_responses;

        (sent_requests, sent_notifications)
    }

    #[cfg(test)]
    /// Returns the subscription stream lag (for testing)
    pub fn get_subscription_stream_lag(&self) -> Option<SubscriptionStreamLag> {
        self.subscription_stream_lag.clone()
    }
}

impl<T> Drop for DataStream<T> {
    /// Terminates the stream by aborting all spawned tasks
    fn drop(&mut self) {
        self.abort_spawned_tasks();
    }
}

impl<T> DataStream<T> {
    /// Aborts all currently spawned tasks. This is useful if the stream is
    /// terminated prematurely, or if the sent data requests are cleared.
    fn abort_spawned_tasks(&mut self) {
        for spawned_task in &self.spawned_tasks {
            spawned_task.abort();
        }
    }
}

/// A simple container to track the start time and lag of a subscription stream
#[derive(Clone, Debug)]
pub struct SubscriptionStreamLag {
    pub start_time: Instant,
    pub time_service: TimeService,
    pub version_lag: u64,
}

impl SubscriptionStreamLag {
    fn new(version_lag: u64, time_service: TimeService) -> Self {
        Self {
            start_time: time_service.now(),
            time_service,
            version_lag,
        }
    }

    /// Returns true iff the subscription stream lag is considered to be
    /// beyond recovery. This occurs when: (i) the stream is lagging for
    /// too long; and (ii) the lag has increased since the last check.
    fn is_beyond_recovery(
        &mut self,
        streaming_service_config: DataStreamingServiceConfig,
        current_stream_lag: u64,
    ) -> bool {
        // Calculate the total duration the stream has been lagging
        let current_time = self.time_service.now();
        let stream_lag_duration = current_time.duration_since(self.start_time);
        let max_stream_lag_duration =
            Duration::from_secs(streaming_service_config.max_subscription_stream_lag_secs);

        // If the lag is further behind and enough time has passed, the stream has failed
        let lag_has_increased = current_stream_lag > self.version_lag;
        let lag_duration_exceeded = stream_lag_duration >= max_stream_lag_duration;
        if lag_has_increased && lag_duration_exceeded {
            return true; // The stream is beyond recovery
        }

        // Otherwise, update the stream lag if we've caught up.
        // This will ensure the lag can only improve.
        if current_stream_lag < self.version_lag {
            self.version_lag = current_stream_lag;
        }

        false // The stream is not yet beyond recovery
    }
}

/// Allows listening to data streams (i.e., streams of data notifications).
#[derive(Debug)]
pub struct DataStreamListener {
    pub data_stream_id: DataStreamId,
    notification_receiver: mpsc::Receiver<DataNotification>,

    /// Stores the number of consecutive timeouts encountered when listening to this stream
    pub num_consecutive_timeouts: u64,
}

impl DataStreamListener {
    pub fn new(
        data_stream_id: DataStreamId,
        notification_receiver: mpsc::Receiver<DataNotification>,
    ) -> Self {
        Self {
            data_stream_id,
            notification_receiver,
            num_consecutive_timeouts: 0,
        }
    }
}

impl Stream for DataStreamListener {
    type Item = DataNotification;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().notification_receiver).poll_next(cx)
    }
}

impl FusedStream for DataStreamListener {
    fn is_terminated(&self) -> bool {
        self.notification_receiver.is_terminated()
    }
}

/// Creates and returns a missing data request if the given client response
/// doesn't satisfy the original request. If the request is satisfied,
/// None is returned.
pub(crate) fn create_missing_data_request(
    data_client_request: &DataClientRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine if the request was satisfied, and if not, create
    // a missing data request to satisfy the original request.
    match data_client_request {
        DataClientRequest::EpochEndingLedgerInfos(request) => {
            create_missing_epoch_ending_ledger_infos_request(request, response_payload)
        },
        DataClientRequest::StateValuesWithProof(request) => {
            create_missing_state_values_request(request, response_payload)
        },
        DataClientRequest::TransactionsWithProof(request) => {
            create_missing_transactions_request(request, response_payload)
        },
        DataClientRequest::TransactionOutputsWithProof(request) => {
            create_missing_transaction_outputs_request(request, response_payload)
        },
        DataClientRequest::TransactionsOrOutputsWithProof(request) => {
            create_missing_transactions_or_outputs_request(request, response_payload)
        },
        _ => Ok(None), // The request was trivially satisfied (based on the type)
    }
}

/// Creates and returns a missing epoch ending ledger info request if the
/// given client response doesn't satisfy the original request. If the request
/// is satisfied, None is returned.
fn create_missing_epoch_ending_ledger_infos_request(
    request: &EpochEndingLedgerInfosRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine the number of requested ledger infos
    let num_requested_ledger_infos = request
        .end_epoch
        .checked_sub(request.start_epoch)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| {
            Error::IntegerOverflow("Number of requested ledger infos has overflown!".into())
        })?;

    // Identify the missing data if the request was not satisfied
    match response_payload {
        ResponsePayload::EpochEndingLedgerInfos(ledger_infos) => {
            // Check if the request was satisfied
            let num_received_ledger_infos = ledger_infos.len() as u64;
            if num_received_ledger_infos < num_requested_ledger_infos {
                let start_epoch = request
                    .start_epoch
                    .checked_add(num_received_ledger_infos)
                    .ok_or_else(|| Error::IntegerOverflow("Start epoch has overflown!".into()))?;
                Ok(Some(DataClientRequest::EpochEndingLedgerInfos(
                    EpochEndingLedgerInfosRequest {
                        start_epoch,
                        end_epoch: request.end_epoch,
                    },
                )))
            } else {
                Ok(None) // The request was satisfied!
            }
        },
        payload => Err(Error::AptosDataClientResponseIsInvalid(format!(
            "Invalid response payload found for epoch ending ledger info request: {:?}",
            payload
        ))),
    }
}

/// Creates and returns a missing state values request if the given client
/// response doesn't satisfy the original request. If the request is satisfied,
/// None is returned.
fn create_missing_state_values_request(
    request: &StateValuesWithProofRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine the number of requested state values
    let num_requested_state_values = request
        .end_index
        .checked_sub(request.start_index)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| {
            Error::IntegerOverflow("Number of requested state values has overflown!".into())
        })?;

    // Identify the missing data if the request was not satisfied
    match response_payload {
        ResponsePayload::StateValuesWithProof(state_values_with_proof) => {
            // Check if the request was satisfied
            let num_received_state_values = state_values_with_proof.raw_values.len() as u64;
            if num_received_state_values < num_requested_state_values {
                let start_index = request
                    .start_index
                    .checked_add(num_received_state_values)
                    .ok_or_else(|| Error::IntegerOverflow("Start index has overflown!".into()))?;
                Ok(Some(DataClientRequest::StateValuesWithProof(
                    StateValuesWithProofRequest {
                        version: request.version,
                        start_index,
                        end_index: request.end_index,
                    },
                )))
            } else {
                Ok(None) // The request was satisfied!
            }
        },
        payload => Err(Error::AptosDataClientResponseIsInvalid(format!(
            "Invalid response payload found for state values request: {:?}",
            payload
        ))),
    }
}

/// Creates and returns a missing transactions request if the given client
/// response doesn't satisfy the original request. If the request is satisfied,
/// None is returned.
fn create_missing_transactions_request(
    request: &TransactionsWithProofRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine the number of requested transactions
    let num_requested_transactions = request
        .end_version
        .checked_sub(request.start_version)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| {
            Error::IntegerOverflow("Number of requested transactions has overflown!".into())
        })?;

    // Identify the missing data if the request was not satisfied
    match response_payload {
        ResponsePayload::TransactionsWithProof(transactions_with_proof) => {
            // Check if the request was satisfied
            let num_received_transactions = transactions_with_proof.get_num_transactions() as u64;
            if num_received_transactions < num_requested_transactions {
                let start_version = request
                    .start_version
                    .checked_add(num_received_transactions)
                    .ok_or_else(|| Error::IntegerOverflow("Start version has overflown!".into()))?;
                Ok(Some(DataClientRequest::TransactionsWithProof(
                    TransactionsWithProofRequest {
                        start_version,
                        end_version: request.end_version,
                        proof_version: request.proof_version,
                        include_events: request.include_events,
                    },
                )))
            } else {
                Ok(None) // The request was satisfied!
            }
        },
        payload => Err(Error::AptosDataClientResponseIsInvalid(format!(
            "Invalid response payload found for transactions request: {:?}",
            payload
        ))),
    }
}

/// Creates and returns a missing transaction outputs request if the given client
/// response doesn't satisfy the original request. If the request is satisfied,
/// None is returned.
fn create_missing_transaction_outputs_request(
    request: &TransactionOutputsWithProofRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine the number of requested transaction outputs
    let num_requested_outputs = request
        .end_version
        .checked_sub(request.start_version)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| {
            Error::IntegerOverflow("Number of requested transaction outputs has overflown!".into())
        })?;

    // Identify the missing data if the request was not satisfied
    match response_payload {
        ResponsePayload::TransactionOutputsWithProof(transaction_outputs_with_proof) => {
            // Check if the request was satisfied
            let num_received_outputs = transaction_outputs_with_proof.get_num_outputs() as u64;
            if num_received_outputs < num_requested_outputs {
                let start_version = request
                    .start_version
                    .checked_add(num_received_outputs)
                    .ok_or_else(|| Error::IntegerOverflow("Start version has overflown!".into()))?;
                Ok(Some(DataClientRequest::TransactionOutputsWithProof(
                    TransactionOutputsWithProofRequest {
                        start_version,
                        end_version: request.end_version,
                        proof_version: request.proof_version,
                    },
                )))
            } else {
                Ok(None) // The request was satisfied!
            }
        },
        payload => Err(Error::AptosDataClientResponseIsInvalid(format!(
            "Invalid response payload found for transaction outputs request: {:?}",
            payload
        ))),
    }
}

/// Creates and returns a missing transactions or outputs request if the
/// given client response doesn't satisfy the original request. If the request
/// is satisfied, None is returned.
fn create_missing_transactions_or_outputs_request(
    request: &TransactionsOrOutputsWithProofRequest,
    response_payload: &ResponsePayload,
) -> Result<Option<DataClientRequest>, Error> {
    // Determine the number of requested transactions or outputs
    let num_request_data_items = request
        .end_version
        .checked_sub(request.start_version)
        .and_then(|v| v.checked_add(1))
        .ok_or_else(|| {
            Error::IntegerOverflow(
                "Number of requested transactions or outputs has overflown!".into(),
            )
        })?;

    // Calculate the number of received data items
    let num_received_data_items = match response_payload {
        ResponsePayload::TransactionsWithProof(transactions_with_proof) => {
            transactions_with_proof.get_num_transactions() as u64
        },
        ResponsePayload::TransactionOutputsWithProof(transaction_outputs_with_proof) => {
            transaction_outputs_with_proof.get_num_outputs() as u64
        },
        payload => {
            return Err(Error::AptosDataClientResponseIsInvalid(format!(
                "Invalid response payload found for transactions or outputs request: {:?}",
                payload
            )))
        },
    };

    // Identify the missing data if the request was not satisfied
    if num_received_data_items < num_request_data_items {
        let start_version = request
            .start_version
            .checked_add(num_received_data_items)
            .ok_or_else(|| Error::IntegerOverflow("Start version has overflown!".into()))?;
        Ok(Some(DataClientRequest::TransactionsOrOutputsWithProof(
            TransactionsOrOutputsWithProofRequest {
                start_version,
                end_version: request.end_version,
                proof_version: request.proof_version,
                include_events: request.include_events,
            },
        )))
    } else {
        Ok(None) // The request was satisfied!
    }
}

/// Returns true iff the data client response payload type matches the
/// expected type of the original request. No other sanity checks are done.
fn sanity_check_client_response_type(
    data_client_request: &DataClientRequest,
    data_client_response: &Response<ResponsePayload>,
) -> bool {
    match data_client_request {
        DataClientRequest::EpochEndingLedgerInfos(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::EpochEndingLedgerInfos(_)
            )
        },
        DataClientRequest::NewTransactionOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionOutputsWithProof(_)
            )
        },
        DataClientRequest::NewTransactionsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionsWithProof(_)
            )
        },
        DataClientRequest::NewTransactionsOrOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionsWithProof(_)
            ) || matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionOutputsWithProof(_)
            )
        },
        DataClientRequest::NumberOfStates(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NumberOfStates(_)
            )
        },
        DataClientRequest::StateValuesWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::StateValuesWithProof(_)
            )
        },
        DataClientRequest::SubscribeTransactionsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionsWithProof(_)
            )
        },
        DataClientRequest::SubscribeTransactionOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionOutputsWithProof(_)
            )
        },
        DataClientRequest::SubscribeTransactionsOrOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionsWithProof(_)
            ) || matches!(
                data_client_response.payload,
                ResponsePayload::NewTransactionOutputsWithProof(_)
            )
        },
        DataClientRequest::TransactionsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::TransactionsWithProof(_)
            )
        },
        DataClientRequest::TransactionOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::TransactionOutputsWithProof(_)
            )
        },
        DataClientRequest::TransactionsOrOutputsWithProof(_) => {
            matches!(
                data_client_response.payload,
                ResponsePayload::TransactionsWithProof(_)
            ) || matches!(
                data_client_response.payload,
                ResponsePayload::TransactionOutputsWithProof(_)
            )
        },
    }
}

/// Transforms the notification feedback into a specific response error that
/// can be sent to the Aptos data client.
fn extract_response_error(
    notification_feedback: &NotificationFeedback,
) -> Result<ResponseError, Error> {
    match notification_feedback {
        NotificationFeedback::InvalidPayloadData => Ok(ResponseError::InvalidData),
        NotificationFeedback::PayloadTypeIsIncorrect => Ok(ResponseError::InvalidPayloadDataType),
        NotificationFeedback::PayloadProofFailed => Ok(ResponseError::ProofVerificationError),
        _ => Err(Error::UnexpectedErrorEncountered(format!(
            "Invalid notification feedback given: {:?}",
            notification_feedback
        ))),
    }
}

fn spawn_request_task<T: AptosDataClientInterface + Send + Clone + 'static>(
    data_stream_id: DataStreamId,
    data_client_request: DataClientRequest,
    aptos_data_client: T,
    pending_response: PendingClientResponse,
    request_timeout_ms: u64,
    stream_update_notifier: aptos_channel::Sender<(), StreamUpdateNotification>,
) -> JoinHandle<()> {
    // Update the requests sent counter
    increment_counter(
        &metrics::SENT_DATA_REQUESTS,
        data_client_request.get_label(),
    );

    // Spawn the request
    tokio::spawn(async move {
        // Time the request (the timer will stop when it's dropped)
        let _timer = start_timer(
            &metrics::DATA_REQUEST_PROCESSING_LATENCY,
            data_client_request.get_label().into(),
        );

        // Fetch the client response
        let client_response = match data_client_request {
            DataClientRequest::EpochEndingLedgerInfos(request) => {
                get_epoch_ending_ledger_infos(aptos_data_client, request, request_timeout_ms).await
            },
            DataClientRequest::NewTransactionsWithProof(request) => {
                get_new_transactions_with_proof(aptos_data_client, request, request_timeout_ms)
                    .await
            },
            DataClientRequest::NewTransactionOutputsWithProof(request) => {
                get_new_transaction_outputs_with_proof(
                    aptos_data_client,
                    request,
                    request_timeout_ms,
                )
                .await
            },
            DataClientRequest::NewTransactionsOrOutputsWithProof(request) => {
                get_new_transactions_or_outputs_with_proof(
                    aptos_data_client,
                    request,
                    request_timeout_ms,
                )
                .await
            },
            DataClientRequest::NumberOfStates(request) => {
                get_number_of_states(aptos_data_client, request, request_timeout_ms).await
            },
            DataClientRequest::StateValuesWithProof(request) => {
                get_states_values_with_proof(aptos_data_client, request, request_timeout_ms).await
            },
            DataClientRequest::SubscribeTransactionsWithProof(request) => {
                subscribe_to_transactions_with_proof(aptos_data_client, request, request_timeout_ms)
                    .await
            },
            DataClientRequest::SubscribeTransactionOutputsWithProof(request) => {
                subscribe_to_transaction_outputs_with_proof(
                    aptos_data_client,
                    request,
                    request_timeout_ms,
                )
                .await
            },
            DataClientRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                subscribe_to_transactions_or_outputs_with_proof(
                    aptos_data_client,
                    request,
                    request_timeout_ms,
                )
                .await
            },
            DataClientRequest::TransactionOutputsWithProof(request) => {
                get_transaction_outputs_with_proof(aptos_data_client, request, request_timeout_ms)
                    .await
            },
            DataClientRequest::TransactionsWithProof(request) => {
                get_transactions_with_proof(aptos_data_client, request, request_timeout_ms).await
            },
            DataClientRequest::TransactionsOrOutputsWithProof(request) => {
                get_transactions_or_outputs_with_proof(
                    aptos_data_client,
                    request,
                    request_timeout_ms,
                )
                .await
            },
        };

        // Increment the appropriate counter depending on the response
        match &client_response {
            Ok(response) => {
                increment_counter(
                    &metrics::RECEIVED_DATA_RESPONSE,
                    response.payload.get_label(),
                );
            },
            Err(error) => {
                increment_counter(&metrics::RECEIVED_RESPONSE_ERROR, error.get_label());
            },
        }

        // Save the response
        pending_response.lock().client_response = Some(client_response);

        // Send a notification via the stream update notifier
        let stream_update_notification = StreamUpdateNotification::new(data_stream_id);
        let _ = stream_update_notifier.push((), stream_update_notification);
    })
}

// TODO: don't drop the v2 response info!

async fn get_states_values_with_proof<T: AptosDataClientInterface + Send + Clone + 'static>(
    aptos_data_client: T,
    request: StateValuesWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_state_values_with_proof(
        request.version,
        request.start_index,
        request.end_index,
        request_timeout_ms,
    );
    client_response
        .await
        .map(|response| response.map(ResponsePayload::from))
}

async fn get_epoch_ending_ledger_infos<T: AptosDataClientInterface + Send + Clone + 'static>(
    aptos_data_client: T,
    request: EpochEndingLedgerInfosRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_epoch_ending_ledger_infos(
        request.start_epoch,
        request.end_epoch,
        request_timeout_ms,
    );
    client_response
        .await
        .map(|response| response.map(ResponsePayload::from))
}

async fn get_new_transaction_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: NewTransactionOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_new_transaction_outputs_with_proof(
        request.known_version,
        request.known_epoch,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, (output_list_with_proof_v2, ledger_info_with_signatures)) =
            response.into_parts();
        let output_list_with_proof = output_list_with_proof_v2.get_output_list_with_proof();
        let response_v1 = Response::new(
            context,
            (output_list_with_proof.clone(), ledger_info_with_signatures),
        );
        response_v1.map(ResponsePayload::from)
    })
}

async fn get_new_transactions_with_proof<T: AptosDataClientInterface + Send + Clone + 'static>(
    aptos_data_client: T,
    request: NewTransactionsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_new_transactions_with_proof(
        request.known_version,
        request.known_epoch,
        request.include_events,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, (transaction_list_with_proof_v2, ledger_info_with_signatures)) =
            response.into_parts();
        let transaction_list_with_proof =
            transaction_list_with_proof_v2.get_transaction_list_with_proof();
        let response_v1 = Response::new(
            context,
            (
                transaction_list_with_proof.clone(),
                ledger_info_with_signatures,
            ),
        );
        response_v1.map(ResponsePayload::from)
    })
}

async fn get_new_transactions_or_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: NewTransactionsOrOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_new_transactions_or_outputs_with_proof(
        request.known_version,
        request.known_epoch,
        request.include_events,
        request_timeout_ms,
    );
    let (
        context,
        ((transaction_list_with_proof_v2, output_list_with_proof_v2), ledger_info_with_signatures),
    ) = client_response.await?.into_parts();
    let transaction_or_output_list_with_proof = (
        transaction_list_with_proof_v2.map(|t| t.get_transaction_list_with_proof().clone()),
        output_list_with_proof_v2.map(|o| o.get_output_list_with_proof().clone()),
    );
    let payload_v1 = (
        transaction_or_output_list_with_proof,
        ledger_info_with_signatures,
    );
    Ok(Response::new(
        context,
        ResponsePayload::try_from(payload_v1)?,
    ))
}

async fn get_number_of_states<T: AptosDataClientInterface + Send + Clone + 'static>(
    aptos_data_client: T,
    request: NumberOfStatesRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response =
        aptos_data_client.get_number_of_states(request.version, request_timeout_ms);
    client_response
        .await
        .map(|response| response.map(ResponsePayload::from))
}

async fn get_transaction_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: TransactionOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_transaction_outputs_with_proof(
        request.proof_version,
        request.start_version,
        request.end_version,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, output_list_with_proof_v2) = response.into_parts();
        let output_list_with_proof = output_list_with_proof_v2.get_output_list_with_proof();
        let response_v1 = Response::new(context, output_list_with_proof.clone());
        response_v1.map(ResponsePayload::from)
    })
}

async fn get_transactions_with_proof<T: AptosDataClientInterface + Send + Clone + 'static>(
    aptos_data_client: T,
    request: TransactionsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_transactions_with_proof(
        request.proof_version,
        request.start_version,
        request.end_version,
        request.include_events,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, transaction_list_with_proof_v2) = response.into_parts();
        let transaction_list_with_proof =
            transaction_list_with_proof_v2.get_transaction_list_with_proof();
        let response_v1 = Response::new(context, transaction_list_with_proof.clone());
        response_v1.map(ResponsePayload::from)
    })
}

async fn get_transactions_or_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: TransactionsOrOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let client_response = aptos_data_client.get_transactions_or_outputs_with_proof(
        request.proof_version,
        request.start_version,
        request.end_version,
        request.include_events,
        request_timeout_ms,
    );
    let (context, (transaction_list_with_proof_v2, output_list_with_proof_v2)) =
        client_response.await?.into_parts();
    let payload_v1 = (
        transaction_list_with_proof_v2.map(|t| t.get_transaction_list_with_proof().clone()),
        output_list_with_proof_v2.map(|o| o.get_output_list_with_proof().clone()),
    );
    Ok(Response::new(
        context,
        ResponsePayload::try_from(payload_v1)?,
    ))
}

async fn subscribe_to_transactions_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: SubscribeTransactionsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let subscription_request_metadata = SubscriptionRequestMetadata {
        known_version_at_stream_start: request.known_version,
        known_epoch_at_stream_start: request.known_epoch,
        subscription_stream_id: request.subscription_stream_id,
        subscription_stream_index: request.subscription_stream_index,
    };
    let client_response = aptos_data_client.subscribe_to_transactions_with_proof(
        subscription_request_metadata,
        request.include_events,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, (transaction_list_with_proof_v2, ledger_info_with_signatures)) =
            response.into_parts();
        let transaction_list_with_proof =
            transaction_list_with_proof_v2.get_transaction_list_with_proof();
        let response_v1 = Response::new(
            context,
            (
                transaction_list_with_proof.clone(),
                ledger_info_with_signatures,
            ),
        );
        response_v1.map(ResponsePayload::from)
    })
}

async fn subscribe_to_transaction_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: SubscribeTransactionOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let subscription_request_metadata = SubscriptionRequestMetadata {
        known_version_at_stream_start: request.known_version,
        known_epoch_at_stream_start: request.known_epoch,
        subscription_stream_id: request.subscription_stream_id,
        subscription_stream_index: request.subscription_stream_index,
    };
    let client_response = aptos_data_client.subscribe_to_transaction_outputs_with_proof(
        subscription_request_metadata,
        request_timeout_ms,
    );
    client_response.await.map(|response| {
        let (context, (output_list_with_proof_v2, ledger_info_with_signatures)) =
            response.into_parts();
        let output_list_with_proof = output_list_with_proof_v2.get_output_list_with_proof();
        let response_v1 = Response::new(
            context,
            (output_list_with_proof.clone(), ledger_info_with_signatures),
        );
        response_v1.map(ResponsePayload::from)
    })
}

async fn subscribe_to_transactions_or_outputs_with_proof<
    T: AptosDataClientInterface + Send + Clone + 'static,
>(
    aptos_data_client: T,
    request: SubscribeTransactionsOrOutputsWithProofRequest,
    request_timeout_ms: u64,
) -> Result<Response<ResponsePayload>, aptos_data_client::error::Error> {
    let subscription_request_metadata = SubscriptionRequestMetadata {
        known_version_at_stream_start: request.known_version,
        known_epoch_at_stream_start: request.known_epoch,
        subscription_stream_id: request.subscription_stream_id,
        subscription_stream_index: request.subscription_stream_index,
    };
    let client_response = aptos_data_client.subscribe_to_transactions_or_outputs_with_proof(
        subscription_request_metadata,
        request.include_events,
        request_timeout_ms,
    );
    let (
        context,
        ((transaction_list_with_proof_v2, output_list_with_proof_v2), ledger_info_with_signatures),
    ) = client_response.await?.into_parts();
    let transaction_or_output_list_with_proof = (
        transaction_list_with_proof_v2.map(|t| t.get_transaction_list_with_proof().clone()),
        output_list_with_proof_v2.map(|o| o.get_output_list_with_proof().clone()),
    );
    let payload_v1 = (
        transaction_or_output_list_with_proof,
        ledger_info_with_signatures,
    );
    Ok(Response::new(
        context,
        ResponsePayload::try_from(payload_v1)?,
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::utils::MockAptosDataClient;
    use aptos_channels::message_queues::QueueStyle;
    use futures::StreamExt;
    use tokio::time::timeout;

    #[tokio::test]
    async fn completed_request_notifies_streaming_service() {
        // Create a data client request
        let data_client_request =
            DataClientRequest::NumberOfStates(NumberOfStatesRequest { version: 0 });

        // Create a mock data client
        let data_client_config = AptosDataClientConfig::default();
        let aptos_data_client =
            MockAptosDataClient::new(data_client_config, true, false, true, true);

        // Create a new pending client response
        let pending_client_response = Arc::new(Mutex::new(Box::new(
            data_notification::PendingClientResponse::new(data_client_request.clone()),
        )));

        // Create a stream update notifier and listener
        let (stream_update_notifier, mut stream_update_listener) =
            aptos_channel::new(QueueStyle::LIFO, 1, None);

        // Verify the request is still pending (the request hasn't been sent yet)
        assert!(pending_client_response.lock().client_response.is_none());

        // Spawn the request task
        let data_stream_id = 10101;
        let join_handle = spawn_request_task(
            data_stream_id,
            data_client_request,
            aptos_data_client,
            pending_client_response.clone(),
            1000,
            stream_update_notifier.clone(),
        );

        // Wait for the request to complete
        join_handle.await.unwrap();

        // Verify the request was completed and we now have a response
        assert!(pending_client_response.lock().client_response.is_some());

        // Verify that a stream update notification is received
        match timeout(Duration::from_secs(5), stream_update_listener.next()).await {
            Ok(Some(stream_update_notification)) => {
                assert_eq!(stream_update_notification.data_stream_id, data_stream_id);
            },
            result => panic!(
                "Stream update notification was not received! Result: {:?}",
                result
            ),
        }
    }
}
