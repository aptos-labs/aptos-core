// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_stream::DataStreamId;
use crate::{data_notification::NotificationId, data_stream::DataStreamListener, error::Error};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use async_trait::async_trait;
use futures::{
    channel::{mpsc, oneshot},
    stream::FusedStream,
    SinkExt, Stream,
};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

pub type Epoch = u64;

/// The streaming client used by state sync to fetch data from the Aptos network
/// to synchronize local state.
///
/// Notes:
/// 1. The streaming service streams data sequentially, so clients (e.g.,
/// state sync) can process data notifications in the order they're received.
/// For example, if we're streaming transactions with proofs, state sync can
/// assume the transactions are returned in monotonically increasing versions.
/// 2. If a stream completes (possibly prematurely), an end of stream
/// notification will be sent to the listener. Once a stream has completed, it
/// is the responsibility of the client to terminate the stream using this API.
#[async_trait]
pub trait DataStreamingClient {
    /// Fetches the state values at the specified version. If `start_index`
    /// is specified, the state values will be fetched starting at the
    /// `start_index` (inclusive). Otherwise, the start index will 0.
    /// The specified version must be an epoch ending version, otherwise an
    /// error will be returned. State proofs are at the same version.
    async fn get_all_state_values(
        &self,
        version: Version,
        start_index: Option<u64>,
    ) -> Result<DataStreamListener, Error>;

    /// Fetches all epoch ending ledger infos starting at `start_epoch`
    /// (inclusive) and ending at the last known epoch advertised in the network.
    async fn get_all_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
    ) -> Result<DataStreamListener, Error>;

    /// Fetches all transaction outputs with proofs from `start_version` to
    /// `end_version` (inclusive) at the specified `proof_version`.
    async fn get_all_transaction_outputs(
        &self,
        start_version: Version,
        end_version: Version,
        proof_version: Version,
    ) -> Result<DataStreamListener, Error>;

    /// Fetches all transactions with proofs from `start_version` to
    /// `end_version` (inclusive) at the specified `proof_version`. If
    /// `include_events` is true, events are also included in the proofs.
    async fn get_all_transactions(
        &self,
        start_version: Version,
        end_version: Version,
        proof_version: Version,
        include_events: bool,
    ) -> Result<DataStreamListener, Error>;

    /// Continuously streams transaction outputs with proofs as the blockchain
    /// grows. The stream starts at `known_version + 1` (inclusive) and
    /// `known_epoch`, where the `known_epoch` is expected to be the epoch
    /// that contains `known_version + 1`, i.e., any epoch change at
    /// `known_version` must be noted by the client.
    /// Transaction output proof versions are tied to ledger infos within the
    /// same epoch, otherwise epoch ending ledger infos will signify epoch changes.
    ///
    /// Note: if a `target` is provided, the stream will terminate once it reaches
    /// the target. Otherwise, it will continue indefinitely.
    async fn continuously_stream_transaction_outputs(
        &self,
        known_version: u64,
        known_epoch: u64,
        target: Option<LedgerInfoWithSignatures>,
    ) -> Result<DataStreamListener, Error>;

    /// Continuously streams transactions with proofs as the blockchain
    /// grows. The stream starts at `known_version + 1` (inclusive) and
    /// `known_epoch`, where the `known_epoch` is expected to be the epoch
    /// that contains `known_version + 1`, i.e., any epoch change at
    /// `known_version` must be noted by the client.
    /// Transaction proof versions are tied to ledger infos within the
    /// same epoch, otherwise epoch ending ledger infos will signify epoch changes.
    ///
    /// If `include_events` is true, events are also included in the proofs.
    ///
    /// Note: if a `target` is provided, the stream will terminate once it reaches
    /// the target. Otherwise, it will continue indefinitely.
    async fn continuously_stream_transactions(
        &self,
        start_version: Version,
        start_epoch: Epoch,
        include_events: bool,
        target: Option<LedgerInfoWithSignatures>,
    ) -> Result<DataStreamListener, Error>;

    /// Terminates the stream with the given stream id and (optionally) provides
    /// feedback about the notification and the termination reason.
    ///
    /// Note:
    /// 1. This is required because: (i) clients must terminate completed
    /// streams (e.g., after receiving an end of stream notification); and (ii)
    /// data payloads may be invalid, e.g., due to malformed data returned by a
    /// misbehaving peer. This notifies the streaming service to terminate the
    /// stream and take any action based on the provided feedback.
    /// 2. Clients that wish to continue fetching data need to open a new stream.
    async fn terminate_stream_with_feedback(
        &self,
        data_stream_id: DataStreamId,
        notification_and_feedback: Option<NotificationAndFeedback>,
    ) -> Result<(), Error>;
}

/// Messages used by the data streaming client for communication with the
/// streaming service. The streaming service will respond to the client request
/// through the given `response_sender`.
#[derive(Debug)]
pub struct StreamRequestMessage {
    pub stream_request: StreamRequest,
    pub response_sender: oneshot::Sender<Result<DataStreamListener, Error>>,
}

/// The data streaming request from the client.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamRequest {
    GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest),
    GetAllStates(GetAllStatesRequest),
    GetAllTransactions(GetAllTransactionsRequest),
    GetAllTransactionOutputs(GetAllTransactionOutputsRequest),
    ContinuouslyStreamTransactions(ContinuouslyStreamTransactionsRequest),
    ContinuouslyStreamTransactionOutputs(ContinuouslyStreamTransactionOutputsRequest),
    TerminateStream(TerminateStreamRequest),
}

impl StreamRequest {
    /// Returns a summary label for the stream request
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::GetAllEpochEndingLedgerInfos(_) => "get_all_epoch_ending_ledger_infos",
            Self::GetAllStates(_) => "get_all_states",
            Self::GetAllTransactions(_) => "get_all_transactions",
            Self::GetAllTransactionOutputs(_) => "get_all_transaction_outputs",
            Self::ContinuouslyStreamTransactions(_) => "continuously_stream_transactions",
            Self::ContinuouslyStreamTransactionOutputs(_) => {
                "continuously_stream_transaction_outputs"
            }
            Self::TerminateStream(_) => "terminate_stream",
        }
    }
}

/// A client request for fetching all available epoch ending ledger infos.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAllEpochEndingLedgerInfosRequest {
    pub start_epoch: Epoch,
}

/// A client request for fetching all states at a specified version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAllStatesRequest {
    pub version: Version,
    pub start_index: u64,
}

/// A client request for fetching all transactions with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAllTransactionsRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub proof_version: Version,
    pub include_events: bool,
}

/// A client request for fetching all transaction outputs with proofs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAllTransactionOutputsRequest {
    pub start_version: Version,
    pub end_version: Version,
    pub proof_version: Version,
}

/// A client request for continuously streaming transactions with proofs
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuouslyStreamTransactionsRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub include_events: bool,
    pub target: Option<LedgerInfoWithSignatures>,
}

/// A client request for continuously streaming transaction outputs with proofs
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContinuouslyStreamTransactionOutputsRequest {
    pub known_version: Version,
    pub known_epoch: Epoch,
    pub target: Option<LedgerInfoWithSignatures>,
}

/// A client request for terminating a stream and providing payload feedback.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminateStreamRequest {
    pub data_stream_id: DataStreamId,
    pub notification_and_feedback: Option<NotificationAndFeedback>,
}

/// The feedback for a given notification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationFeedback {
    EmptyPayloadData,
    EndOfStream,
    InvalidPayloadData,
    PayloadProofFailed,
    PayloadTypeIsIncorrect,
}

impl NotificationFeedback {
    /// Returns a summary label for the notification feedback
    pub fn get_label(&self) -> &'static str {
        match self {
            Self::EmptyPayloadData => "empty_payload_data",
            Self::EndOfStream => "end_of_stream",
            Self::InvalidPayloadData => "invalid_payload_data",
            Self::PayloadProofFailed => "payload_proof_failed",
            Self::PayloadTypeIsIncorrect => "payload_type_is_correct",
        }
    }
}

/// The streaming service client that talks to the streaming service.
#[derive(Clone)]
pub struct StreamingServiceClient {
    request_sender: mpsc::UnboundedSender<StreamRequestMessage>,
}

impl StreamingServiceClient {
    pub fn new(request_sender: mpsc::UnboundedSender<StreamRequestMessage>) -> Self {
        Self { request_sender }
    }

    async fn send_stream_request(
        &self,
        client_request: StreamRequest,
    ) -> Result<oneshot::Receiver<Result<DataStreamListener, Error>>, Error> {
        let mut request_sender = self.request_sender.clone();
        let (response_sender, response_receiver) = oneshot::channel();
        let request_message = StreamRequestMessage {
            stream_request: client_request,
            response_sender,
        };
        request_sender.send(request_message).await?;

        Ok(response_receiver)
    }

    async fn send_request_and_await_response(
        &self,
        client_request: StreamRequest,
    ) -> Result<DataStreamListener, Error> {
        let response_receiver = self.send_stream_request(client_request).await?;
        response_receiver.await?
    }
}

#[async_trait]
impl DataStreamingClient for StreamingServiceClient {
    async fn get_all_state_values(
        &self,
        version: u64,
        start_index: Option<u64>,
    ) -> Result<DataStreamListener, Error> {
        let start_index = start_index.unwrap_or(0);
        let client_request = StreamRequest::GetAllStates(GetAllStatesRequest {
            version,
            start_index,
        });
        self.send_request_and_await_response(client_request).await
    }

    async fn get_all_epoch_ending_ledger_infos(
        &self,
        start_epoch: u64,
    ) -> Result<DataStreamListener, Error> {
        let client_request =
            StreamRequest::GetAllEpochEndingLedgerInfos(GetAllEpochEndingLedgerInfosRequest {
                start_epoch,
            });
        self.send_request_and_await_response(client_request).await
    }

    async fn get_all_transaction_outputs(
        &self,
        start_version: u64,
        end_version: u64,
        proof_version: u64,
    ) -> Result<DataStreamListener, Error> {
        let client_request =
            StreamRequest::GetAllTransactionOutputs(GetAllTransactionOutputsRequest {
                start_version,
                end_version,
                proof_version,
            });
        self.send_request_and_await_response(client_request).await
    }

    async fn get_all_transactions(
        &self,
        start_version: u64,
        end_version: u64,
        proof_version: u64,
        include_events: bool,
    ) -> Result<DataStreamListener, Error> {
        let client_request = StreamRequest::GetAllTransactions(GetAllTransactionsRequest {
            start_version,
            end_version,
            proof_version,
            include_events,
        });
        self.send_request_and_await_response(client_request).await
    }

    async fn continuously_stream_transaction_outputs(
        &self,
        known_version: u64,
        known_epoch: u64,
        target: Option<LedgerInfoWithSignatures>,
    ) -> Result<DataStreamListener, Error> {
        let client_request = StreamRequest::ContinuouslyStreamTransactionOutputs(
            ContinuouslyStreamTransactionOutputsRequest {
                known_version,
                known_epoch,
                target,
            },
        );
        self.send_request_and_await_response(client_request).await
    }

    async fn continuously_stream_transactions(
        &self,
        known_version: u64,
        known_epoch: u64,
        include_events: bool,
        target: Option<LedgerInfoWithSignatures>,
    ) -> Result<DataStreamListener, Error> {
        let client_request =
            StreamRequest::ContinuouslyStreamTransactions(ContinuouslyStreamTransactionsRequest {
                known_version,
                known_epoch,
                include_events,
                target,
            });
        self.send_request_and_await_response(client_request).await
    }

    async fn terminate_stream_with_feedback(
        &self,
        data_stream_id: DataStreamId,
        notification_and_feedback: Option<NotificationAndFeedback>,
    ) -> Result<(), Error> {
        let client_request = StreamRequest::TerminateStream(TerminateStreamRequest {
            data_stream_id,
            notification_and_feedback,
        });
        // We can ignore the receiver as no data will be sent.
        let _ = self.send_stream_request(client_request).await?;
        Ok(())
    }
}

/// The component that enables listening to requests from streaming service
/// clients (e.g., state sync).
#[derive(Debug)]
pub struct StreamingServiceListener {
    request_receiver: mpsc::UnboundedReceiver<StreamRequestMessage>,
}

impl StreamingServiceListener {
    pub fn new(request_receiver: mpsc::UnboundedReceiver<StreamRequestMessage>) -> Self {
        Self { request_receiver }
    }
}

impl Stream for StreamingServiceListener {
    type Item = StreamRequestMessage;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().request_receiver).poll_next(cx)
    }
}

impl FusedStream for StreamingServiceListener {
    fn is_terminated(&self) -> bool {
        self.request_receiver.is_terminated()
    }
}

/// A simple container that allows clients to specify feedback
/// for a notification they received.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NotificationAndFeedback {
    pub notification_id: NotificationId,
    pub notification_feedback: NotificationFeedback,
}

impl NotificationAndFeedback {
    pub fn new(
        notification_id: NotificationId,
        notification_feedback: NotificationFeedback,
    ) -> Self {
        Self {
            notification_id,
            notification_feedback,
        }
    }
}

/// This method returns a (StreamingServiceClient, StreamingServiceListener) pair that can be used
/// to allow clients to make requests to the streaming service.
pub fn new_streaming_service_client_listener_pair(
) -> (StreamingServiceClient, StreamingServiceListener) {
    let (request_sender, request_listener) = mpsc::unbounded();

    let streaming_service_client = StreamingServiceClient::new(request_sender);
    let streaming_service_listener = StreamingServiceListener::new(request_listener);

    (streaming_service_client, streaming_service_listener)
}
