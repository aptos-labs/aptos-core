// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_stream::{DataStream, DataStreamId, DataStreamListener},
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    streaming_client::{
        StreamRequest, StreamRequestMessage, StreamingServiceListener, TerminateStreamRequest,
    },
};
use aptos_config::config::DataStreamingServiceConfig;
use aptos_data_client::{AptosDataClient, GlobalDataSummary, OptimalChunkSizes};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_logger::prelude::*;
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

// Useful constants for the Data Streaming Service
const GLOBAL_DATA_REFRESH_LOG_FREQ_SECS: u64 = 3;
const NO_DATA_TO_FETCH_LOG_FREQ_SECS: u64 = 3;
const STREAM_REQUEST_ERROR_LOG_FREQ_SECS: u64 = 3;
const TERMINATE_NO_FEEDBACK: &str = "no_feedback";

/// The data streaming service that responds to data stream requests.
pub struct DataStreamingService<T> {
    // The configuration for this streaming service.
    config: DataStreamingServiceConfig,

    // The data client through which to fetch data from the Aptos network
    aptos_data_client: T,

    // Cached global data summary
    global_data_summary: GlobalDataSummary,

    // All requested data streams from clients
    data_streams: HashMap<DataStreamId, DataStream<T>>,

    // The listener through which to hear new client stream requests
    stream_requests: StreamingServiceListener,

    // Unique ID generators to maintain unique IDs across streams
    stream_id_generator: U64IdGenerator,
    notification_id_generator: Arc<U64IdGenerator>,
}

impl<T: AptosDataClient + Send + Clone + 'static> DataStreamingService<T> {
    pub fn new(
        config: DataStreamingServiceConfig,
        aptos_data_client: T,
        stream_requests: StreamingServiceListener,
    ) -> Self {
        Self {
            config,
            aptos_data_client,
            global_data_summary: GlobalDataSummary::empty(),
            data_streams: HashMap::new(),
            stream_requests,
            stream_id_generator: U64IdGenerator::new(),
            notification_id_generator: Arc::new(U64IdGenerator::new()),
        }
    }

    /// Starts the dedicated streaming service
    pub async fn start_service(mut self) {
        let mut data_refresh_interval = IntervalStream::new(interval(Duration::from_millis(
            self.config.global_summary_refresh_interval_ms,
        )))
        .fuse();
        let mut progress_check_interval = IntervalStream::new(interval(Duration::from_millis(
            self.config.progress_check_interval_ms,
        )))
        .fuse();

        loop {
            ::futures::select! {
                stream_request = self.stream_requests.select_next_some() => {
                    self.handle_stream_request_message(stream_request);
                }
                _ = data_refresh_interval.select_next_some() => {
                    self.refresh_global_data_summary();
                }
                _ = progress_check_interval.select_next_some() => {
                    self.check_progress_of_all_data_streams().await;
                }
            }
        }
    }

    /// Handles new stream request messages from clients
    fn handle_stream_request_message(&mut self, request_message: StreamRequestMessage) {
        if let StreamRequest::TerminateStream(request) = request_message.stream_request {
            // Process the feedback request
            if let Err(error) = self.process_terminate_stream_request(&request) {
                error!(LogSchema::new(LogEntry::HandleTerminateRequest)
                    .event(LogEvent::Error)
                    .error(&error));
            }
            return;
        }

        // Process the stream request
        let response = self.process_new_stream_request(&request_message);
        if let Err(error) = &response {
            sample!(
                SampleRate::Duration(Duration::from_secs(STREAM_REQUEST_ERROR_LOG_FREQ_SECS)),
                error!(LogSchema::new(LogEntry::HandleStreamRequest)
                    .event(LogEvent::Error)
                    .error(error));
            );
        }

        // Send the response to the client
        if let Err(error) = request_message.response_sender.send(response) {
            error!(LogSchema::new(LogEntry::RespondToStreamRequest)
                .event(LogEvent::Error)
                .message(&format!(
                    "Failed to send response for stream request: {:?}",
                    error
                )));
        }
    }

    /// Processes a request for terminating a data stream.
    /// TODO(joshlind): once this is exposed to the wild, we'll need automatic
    /// garbage collection for misbehaving clients.
    fn process_terminate_stream_request(
        &mut self,
        terminate_request: &TerminateStreamRequest,
    ) -> Result<(), Error> {
        // Grab the stream id and feedback
        let data_stream_id = &terminate_request.data_stream_id;
        let notification_and_feedback = &terminate_request.notification_and_feedback;

        // Increment the stream termination counter
        let feedback_label = match notification_and_feedback {
            Some(notification_and_feedback) => {
                notification_and_feedback.notification_feedback.get_label()
            }
            None => TERMINATE_NO_FEEDBACK,
        };
        metrics::increment_counter(&metrics::TERMINATE_DATA_STREAM, feedback_label);

        // Remove the data stream
        if let Some(data_stream) = self.data_streams.remove(data_stream_id) {
            info!(LogSchema::new(LogEntry::HandleTerminateRequest)
                .stream_id(*data_stream_id)
                .event(LogEvent::Success)
                .message(&format!(
                    "Terminating the data stream with ID: {:?}. Notification and feedback: {:?}",
                    data_stream_id, notification_and_feedback,
                )));

            // Handle any notification feedback
            if let Some(notification_and_feedback) = notification_and_feedback {
                let notification_id = &notification_and_feedback.notification_id;
                let feedback = &notification_and_feedback.notification_feedback;
                if data_stream.sent_notification(notification_id) {
                    data_stream.handle_notification_feedback(notification_id, feedback)?;
                    Ok(())
                } else {
                    Err(Error::UnexpectedErrorEncountered(format!(
                        "Data stream ID: {:?} did not appear to send notification ID: {:?}",
                        data_stream_id, notification_id,
                    )))
                }
            } else {
                Ok(())
            }
        } else {
            Err(Error::UnexpectedErrorEncountered(format!(
                "Unable to find data stream with ID: {:?}. Notification and feedback: {:?}",
                data_stream_id, notification_and_feedback,
            )))
        }
    }

    /// Creates a new stream and ensures the data for that stream is available
    fn process_new_stream_request(
        &mut self,
        request_message: &StreamRequestMessage,
    ) -> Result<DataStreamListener, Error> {
        // Increment the stream creation counter
        metrics::increment_counter(
            &metrics::CREATE_DATA_STREAM,
            request_message.stream_request.get_label(),
        );

        // Refresh the cached global data summary
        self.refresh_global_data_summary();

        // Create a new data stream
        let stream_id = self.stream_id_generator.next();
        let (data_stream, stream_listener) = DataStream::new(
            self.config,
            stream_id,
            &request_message.stream_request,
            self.aptos_data_client.clone(),
            self.notification_id_generator.clone(),
            &self.global_data_summary.advertised_data,
        )?;

        // Verify the data stream can be fulfilled using the currently advertised data
        data_stream.ensure_data_is_available(&self.global_data_summary.advertised_data)?;

        // Store the data stream internally
        if self.data_streams.insert(stream_id, data_stream).is_some() {
            panic!(
                "Duplicate data stream found! This should not occur! ID: {:?}",
                stream_id,
            );
        }
        info!(LogSchema::new(LogEntry::HandleStreamRequest)
            .stream_id(stream_id)
            .event(LogEvent::Success)
            .message(&format!(
                "Stream created for request: {:?}",
                request_message
            )));

        // Return the listener
        Ok(stream_listener)
    }

    /// Refreshes the global data summary by communicating with the Aptos data client
    fn refresh_global_data_summary(&mut self) {
        if let Err(error) = self.fetch_global_data_summary() {
            metrics::increment_counter(&metrics::GLOBAL_DATA_SUMMARY_ERROR, error.get_label());
            sample!(
                SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_REFRESH_LOG_FREQ_SECS)),
                error!(LogSchema::new(LogEntry::RefreshGlobalData)
                    .event(LogEvent::Error)
                    .error(&error))
            );
        }
    }

    fn fetch_global_data_summary(&mut self) -> Result<(), Error> {
        let global_data_summary = self.aptos_data_client.get_global_data_summary();
        if global_data_summary.is_empty() {
            sample!(
                SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_REFRESH_LOG_FREQ_SECS)),
                info!(LogSchema::new(LogEntry::RefreshGlobalData)
                    .message("Latest global data summary is empty."))
            );
        } else {
            verify_optimal_chunk_sizes(&global_data_summary.optimal_chunk_sizes)?;
            self.global_data_summary = global_data_summary;
        }

        Ok(())
    }

    /// Ensures that all existing data streams are making progress
    async fn check_progress_of_all_data_streams(&mut self) {
        // Drive the progress of each stream
        let data_stream_ids = self.get_all_data_stream_ids();
        for data_stream_id in &data_stream_ids {
            if let Err(error) = self.update_progress_of_data_stream(data_stream_id).await {
                if matches!(error, Error::NoDataToFetch(_)) {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(NO_DATA_TO_FETCH_LOG_FREQ_SECS)),
                        info!(LogSchema::new(LogEntry::CheckStreamProgress)
                            .stream_id(*data_stream_id)
                            .event(LogEvent::Pending)
                            .error(&error))
                    );
                } else {
                    metrics::increment_counter(
                        &metrics::CHECK_STREAM_PROGRESS_ERROR,
                        error.get_label(),
                    );
                    error!(LogSchema::new(LogEntry::CheckStreamProgress)
                        .stream_id(*data_stream_id)
                        .event(LogEvent::Error)
                        .error(&error));
                }
            }
        }

        // Update the metrics
        metrics::set_active_data_streams(data_stream_ids.len());
    }

    /// Ensures that a data stream has in-flight data requests and handles
    /// any new responses that have arrived since we last checked.
    async fn update_progress_of_data_stream(
        &mut self,
        data_stream_id: &DataStreamId,
    ) -> Result<(), Error> {
        let global_data_summary = self.global_data_summary.clone();

        // If there was a send failure, terminate the stream
        let data_stream = self.get_data_stream(data_stream_id);
        if data_stream.send_failure() {
            info!(
                (LogSchema::new(LogEntry::TerminateStream)
                    .stream_id(*data_stream_id)
                    .event(LogEvent::Success)
                    .message("There was a send failure, terminating the stream."))
            );
            metrics::DATA_STREAM_SEND_FAILURE.inc();
            if self.data_streams.remove(data_stream_id).is_none() {
                return Err(Error::UnexpectedErrorEncountered(format!(
                    "Failed to terminate stream id {:?} for send failure! Stream not found.",
                    data_stream_id
                )));
            }
            return Ok(());
        }

        // Drive data stream progress
        if !data_stream.data_requests_initialized() {
            // Initialize the request batch by sending out data client requests
            data_stream.initialize_data_requests(global_data_summary)?;
            info!(
                (LogSchema::new(LogEntry::InitializeStream)
                    .stream_id(*data_stream_id)
                    .event(LogEvent::Success)
                    .message("Data stream initialized."))
            );
        } else {
            // Process any data client requests that have received responses
            data_stream
                .process_data_responses(global_data_summary)
                .await?;
        }

        Ok(())
    }

    fn get_all_data_stream_ids(&self) -> Vec<DataStreamId> {
        self.data_streams
            .keys()
            .cloned()
            .collect::<Vec<DataStreamId>>()
    }

    /// Returns the data stream associated with the given `data_stream_id`.
    /// Note: this method assumes the caller has already verified the stream exists.
    fn get_data_stream(&mut self, data_stream_id: &DataStreamId) -> &mut DataStream<T> {
        self.data_streams
            .get_mut(data_stream_id)
            .unwrap_or_else(|| {
                panic!(
                    "Expected a data stream with ID: {:?}, but found None!",
                    data_stream_id
                )
            })
    }
}

/// Verifies that all optimal chunk sizes are valid (i.e., not zero). Returns an
/// error if a chunk size is 0.
fn verify_optimal_chunk_sizes(optimal_chunk_sizes: &OptimalChunkSizes) -> Result<(), Error> {
    if optimal_chunk_sizes.state_chunk_size == 0
        || optimal_chunk_sizes.epoch_chunk_size == 0
        || optimal_chunk_sizes.transaction_chunk_size == 0
        || optimal_chunk_sizes.transaction_output_chunk_size == 0
    {
        Err(Error::AptosDataClientResponseIsInvalid(format!(
            "Found at least one optimal chunk size of zero: {:?}",
            optimal_chunk_sizes
        )))
    } else {
        Ok(())
    }
}

/// Unit tests for the streaming service. We place these here to inspect
/// the internal state of the object.
#[cfg(test)]
mod streaming_service_tests {
    use crate::data_stream::{DataStreamId, DataStreamListener};
    use crate::error::Error;
    use crate::streaming_client::{
        GetAllStatesRequest, NotificationAndFeedback, NotificationFeedback, StreamRequest,
        StreamRequestMessage, TerminateStreamRequest,
    };
    use crate::tests;
    use crate::tests::utils::MIN_ADVERTISED_STATES;
    use futures::channel::oneshot;
    use futures::channel::oneshot::Receiver;
    use futures::FutureExt;
    use futures::StreamExt;
    use std::ops::Add;
    use std::time::{Duration, Instant};
    use tokio::time::timeout;

    const MAX_STREAM_WAIT_SECS: u64 = 60;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_drop_data_streams() {
        // Create a new streaming service
        let (_, mut streaming_service) =
            tests::streaming_service::create_streaming_client_and_server(false, false, true);

        // Create multiple data streams
        let num_data_streams = 10;
        let mut stream_ids = vec![];
        for _ in 0..num_data_streams {
            // Create a new data stream
            let (new_stream_request, response_receiver) = create_new_stream_request();
            streaming_service.handle_stream_request_message(new_stream_request);
            let data_stream_listener = response_receiver.now_or_never().unwrap().unwrap().unwrap();
            let data_stream_id = data_stream_listener.data_stream_id;

            // Remember the data stream id and drop the listener
            stream_ids.push(data_stream_id);
        }

        // Verify the number of active data streams
        assert_eq!(
            streaming_service.get_all_data_stream_ids().len(),
            num_data_streams
        );

        // Drive progress of the streaming service (the streaming service
        // should detect the dropped listeners and remove the streams).
        let timeout_deadline = Instant::now().add(Duration::from_secs(MAX_STREAM_WAIT_SECS));
        while Instant::now() < timeout_deadline {
            streaming_service.check_progress_of_all_data_streams().await;
            if streaming_service.get_all_data_stream_ids().is_empty() {
                return; // All streams were dropped!
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("The streaming service failed to drop the data streams!");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_terminate_data_streams() {
        // Create a new streaming service
        let (_, mut streaming_service) =
            tests::streaming_service::create_streaming_client_and_server(false, false, true);

        // Verify there are no data streams
        assert!(streaming_service.get_all_data_stream_ids().is_empty());

        // Create multiple data streams
        let num_data_streams = 10;
        let mut stream_ids_and_listeners = vec![];
        for _ in 0..num_data_streams {
            // Create a new data stream
            let (new_stream_request, response_receiver) = create_new_stream_request();
            streaming_service.handle_stream_request_message(new_stream_request);
            let data_stream_listener = response_receiver.now_or_never().unwrap().unwrap().unwrap();
            let data_stream_id = data_stream_listener.data_stream_id;

            // Verify the data stream is actively held by the streaming service
            let all_data_stream_ids = streaming_service.get_all_data_stream_ids();
            assert!(all_data_stream_ids.contains(&data_stream_id));

            // Remember the data stream id and listener
            stream_ids_and_listeners.push((data_stream_id, data_stream_listener));
        }

        // Verify the number of active data streams
        assert_eq!(
            streaming_service.get_all_data_stream_ids().len(),
            num_data_streams
        );

        // Try to terminate a data stream with an incorrect ID and verify
        // an error is returned.
        let terminate_stream_request = TerminateStreamRequest {
            data_stream_id: 1919123,
            notification_and_feedback: None,
        };
        streaming_service
            .process_terminate_stream_request(&terminate_stream_request)
            .unwrap_err();

        // Terminate all the streams and verify they're no longer held
        for (data_stream_id, _) in stream_ids_and_listeners {
            // Terminate the data stream (with no feedback)
            let (terminate_stream_request, _) =
                create_terminate_stream_request(data_stream_id, None);
            streaming_service.handle_stream_request_message(terminate_stream_request);

            // Verify the stream has been removed
            let all_data_stream_ids = streaming_service.get_all_data_stream_ids();
            assert!(!all_data_stream_ids.contains(&data_stream_id));
        }

        // Verify there are no data streams
        assert!(streaming_service.get_all_data_stream_ids().is_empty());
    }

    #[tokio::test]
    async fn test_terminate_data_streams_feedback() {
        // Verify stream termination even if invalid feedback is given (i.e., id mismatch)
        for invalid_feedback in [false, true] {
            // Create a new streaming service
            let (_, mut streaming_service) =
                tests::streaming_service::create_streaming_client_and_server(false, false, true);

            // Create multiple data streams
            let num_data_streams = 10;
            let mut stream_ids_and_listeners = vec![];
            for _ in 0..num_data_streams {
                // Create a new data stream
                let (new_stream_request, response_receiver) = create_new_stream_request();
                streaming_service.handle_stream_request_message(new_stream_request);
                let data_stream_listener =
                    response_receiver.now_or_never().unwrap().unwrap().unwrap();
                let data_stream_id = data_stream_listener.data_stream_id;

                // Remember the data stream id and listener
                stream_ids_and_listeners.push((data_stream_id, data_stream_listener));
            }

            // Fetch a notification from each data stream and terminate the stream
            for (data_stream_id, data_stream_listener) in &mut stream_ids_and_listeners {
                let timeout_deadline =
                    Instant::now().add(Duration::from_secs(MAX_STREAM_WAIT_SECS));
                while Instant::now() < timeout_deadline {
                    streaming_service.check_progress_of_all_data_streams().await;
                    if let Ok(data_notification) = timeout(
                        Duration::from_secs(1),
                        data_stream_listener.select_next_some(),
                    )
                    .await
                    {
                        // Terminate the data stream
                        let notification_id = if invalid_feedback {
                            10101010 // Invalid notification id
                        } else {
                            data_notification.notification_id
                        };
                        let notification_and_feedback = Some(NotificationAndFeedback {
                            notification_id,
                            notification_feedback: NotificationFeedback::InvalidPayloadData,
                        });
                        let (terminate_stream_request, _) = create_terminate_stream_request(
                            *data_stream_id,
                            notification_and_feedback,
                        );
                        streaming_service.handle_stream_request_message(terminate_stream_request);

                        // Verify the stream has been removed
                        let all_data_stream_ids = streaming_service.get_all_data_stream_ids();
                        assert!(!all_data_stream_ids.contains(data_stream_id));
                        break;
                    }
                }
            }

            // Verify there are no data streams
            assert!(streaming_service.get_all_data_stream_ids().is_empty());
        }
    }

    /// Creates a new stream request message for state values
    fn create_new_stream_request() -> (
        StreamRequestMessage,
        Receiver<Result<DataStreamListener, Error>>,
    ) {
        let stream_request = StreamRequest::GetAllStates(GetAllStatesRequest {
            version: MIN_ADVERTISED_STATES,
            start_index: 0,
        });
        create_request_message_and_receiver(stream_request)
    }

    /// Creates a new terminate stream request message
    fn create_terminate_stream_request(
        data_stream_id: DataStreamId,
        notification_and_feedback: Option<NotificationAndFeedback>,
    ) -> (
        StreamRequestMessage,
        Receiver<Result<DataStreamListener, Error>>,
    ) {
        let stream_request = StreamRequest::TerminateStream(TerminateStreamRequest {
            data_stream_id,
            notification_and_feedback,
        });
        create_request_message_and_receiver(stream_request)
    }

    /// Creates a new stream request message and response receiver
    fn create_request_message_and_receiver(
        stream_request: StreamRequest,
    ) -> (
        StreamRequestMessage,
        Receiver<Result<DataStreamListener, Error>>,
    ) {
        let (response_sender, response_receiver) = oneshot::channel();
        let request_message = StreamRequestMessage {
            stream_request,
            response_sender,
        };
        (request_message, response_receiver)
    }
}
