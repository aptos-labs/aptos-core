// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    availability_checks::ensure_data_is_available,
    data_stream::{DataStream, DataStreamId, DataStreamListener},
    error::Error,
    stream_progress_tracker::StreamProgressTracker,
    streaming_client::{StreamRequestMessage, StreamingServiceListener},
};
use diem_data_client::{DataClientPayload, DiemDataClient, GlobalDataSummary};
use futures::StreamExt;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

/// The data streaming service that responds to data stream requests.
pub struct DataStreamingService<T> {
    // The data client through which to fetch data from the Diem network
    diem_data_client: T,

    // Cached global data summary
    global_data_summary: GlobalDataSummary,

    // All requested data streams from clients
    data_streams: HashMap<DataStreamId, DataStream>,

    // The listener through which to hear new client stream requests
    stream_requests: StreamingServiceListener,

    // Unique ID generators to maintain unique IDs across streams
    next_stream_id: AtomicU64,
    next_notification_id: AtomicU64,
}

impl<T: DiemDataClient> DataStreamingService<T> {
    pub fn new(diem_data_client: T, stream_requests: StreamingServiceListener) -> Self {
        Self {
            diem_data_client,
            global_data_summary: GlobalDataSummary::empty(),
            data_streams: HashMap::new(),
            stream_requests,
            next_stream_id: AtomicU64::new(0),
            next_notification_id: AtomicU64::new(0),
        }
    }

    pub async fn start_service(mut self) {
        loop {
            ::futures::select! {
                stream_request = self.stream_requests.select_next_some() => {
                    self.handle_stream_request_message(stream_request);
                }
            }
        }
    }

    fn handle_stream_request_message(&mut self, request_message: StreamRequestMessage) {
        // Process the request message
        let response = self.process_new_stream_request(&request_message);

        // Send the response to the client
        if let Err(_error) = request_message.response_sender.send(response) {
            // TODO(joshlind): once we support logging, log this error!
        }
    }

    fn process_new_stream_request(
        &mut self,
        request_message: &StreamRequestMessage,
    ) -> Result<DataStreamListener, Error> {
        // Refresh the cached global data summary
        self.refresh_global_data_summary()?;

        // Create a new stream progress tracker
        let advertised_data = &self.global_data_summary.advertised_data;
        let stream_progress_tracker =
            StreamProgressTracker::new(&request_message.stream_request, advertised_data)?;

        // Verify the data stream can be fulfilled
        ensure_data_is_available(&stream_progress_tracker, advertised_data)?;

        // Create a new data stream and unique ID
        let stream_id = self.next_stream_id.fetch_add(1, Ordering::Relaxed);
        let (data_stream, stream_listener) = DataStream::new(stream_progress_tracker);

        // Store the data stream internally
        if let Some(existing_stream) = self.data_streams.insert(stream_id, data_stream) {
            panic!(
                "Duplicate data stream found! This should not occur! ID: {}, existing data stream: {:?}",
                stream_id, existing_stream
            );
        }

        // Return the listener
        Ok(stream_listener)
    }

    fn refresh_global_data_summary(&mut self) -> Result<(), Error> {
        match self.diem_data_client.get_global_data_summary() {
            Ok(data_client_response) => {
                match data_client_response.response_payload {
                    DataClientPayload::GlobalDataSummary(global_data_summary) => {
                        self.global_data_summary = global_data_summary;
                        Ok(())
                    },
                    result => {
                        Err(Error::DiemDataClientResponseIsInvalid(format!(
                            "Response payload type is incorrect! Expected a global data summary, but got: {:?}",
                            result
                        )))
                    }
                }
            }
            Err(error) => {
                Err(Error::DiemDataClientResponseIsInvalid(format!(
                    "Failed to fetch global data summary! Error: {:?}",
                    error
                )))
            }
        }
    }
}
