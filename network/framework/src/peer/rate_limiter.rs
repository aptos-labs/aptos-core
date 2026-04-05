// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters,
    peer_manager::PeerManagerError,
    protocols::{
        stream::StreamMessage,
        wire::messaging::v1::{MultiplexMessage, ReadError},
    },
};
use aptos_logger::debug;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_token_bucket::TokenBucket;

// Label constants for the two token bucket types
const MESSAGES_BUCKET_LABEL: &str = "messages";
const BYTES_BUCKET_LABEL: &str = "bytes";

/// A simple inbound rate limiter backed by token buckets for bytes and messages
pub struct InboundMessageRateLimiter {
    bytes_bucket: Option<TokenBucket>,
    messages_bucket: Option<TokenBucket>,
    time_service: TimeService,
}

impl InboundMessageRateLimiter {
    /// Creates a new rate limiter with the given message and byte limits
    pub fn new(
        messages_per_sec: Option<u64>,
        bytes_per_sec: Option<u64>,
        time_service: TimeService,
    ) -> Option<Self> {
        // If both limits are missing, return none
        if bytes_per_sec.is_none() && messages_per_sec.is_none() {
            return None;
        }

        // Otherwise, create token buckets for the provided limits
        let messages_bucket = messages_per_sec.map(|messages| {
            TokenBucket::new(
                messages, // Initial capacity
                messages, // Refill rate per second
                time_service.clone(),
            )
        });
        let bytes_bucket = bytes_per_sec.map(|bytes| {
            TokenBucket::new(
                bytes, // Initial capacity
                bytes, // Refill rate per second
                time_service.clone(),
            )
        });

        Some(Self {
            bytes_bucket,
            messages_bucket,
            time_service,
        })
    }

    /// Sleeps until both token buckets can handle the given multiplex message
    pub async fn throttle(
        &mut self,
        message: &Result<MultiplexMessage, ReadError>,
    ) -> Result<(), PeerManagerError> {
        // Calculate the message count and bytes for the given message
        let (message_count, message_bytes) = get_message_count_and_bytes(message);

        // Verify we have enough tokens to process the message
        if let Some(messages_bucket) = &mut self.messages_bucket {
            if message_count > 0 {
                wait_for_tokens(
                    messages_bucket,
                    message_count,
                    &self.time_service,
                    MESSAGES_BUCKET_LABEL,
                )
                .await?;
            }
        }
        if let Some(bytes_bucket) = &mut self.bytes_bucket {
            if message_bytes > 0 {
                wait_for_tokens(
                    bytes_bucket,
                    message_bytes,
                    &self.time_service,
                    BYTES_BUCKET_LABEL,
                )
                .await?;
            }
        }

        Ok(())
    }
}

/// Returns the message count and bytes for the given multiplex message
fn get_message_count_and_bytes(message: &Result<MultiplexMessage, ReadError>) -> (u64, u64) {
    // Calculate the number of messages (required to handle stream messages)
    let message_count: u64 = match message {
        Ok(MultiplexMessage::Message(_)) => 1,
        Ok(MultiplexMessage::Stream(StreamMessage::Header(_))) => 1,
        Ok(MultiplexMessage::Stream(StreamMessage::Fragment(_))) => 0,
        Err(_) => 0,
    };

    // Calculate the number of bytes in the message
    let message_bytes: u64 = match message {
        Ok(MultiplexMessage::Message(msg)) => msg.data_len() as u64,
        Ok(MultiplexMessage::Stream(StreamMessage::Header(header))) => {
            header.message.data_len() as u64
        },
        Ok(MultiplexMessage::Stream(StreamMessage::Fragment(fragment))) => {
            fragment.raw_data.len() as u64
        },
        Err(_) => 0,
    };

    (message_count, message_bytes)
}

/// Loops until the bucket has enough tokens, sleeping between attempts
async fn wait_for_tokens(
    bucket: &mut TokenBucket,
    requested: u64,
    time_service: &TimeService,
    bucket_label: &'static str,
) -> Result<(), PeerManagerError> {
    loop {
        // Attempt to acquire the tokens
        let result = bucket.try_acquire_all(requested);

        // Process the acquisition result
        match result {
            Ok(()) => return Ok(()),
            Err(Some(ready_at_time)) => {
                debug!(
                    "Failed to acquire {} tokens from the {} bucket. Throttling until {:?}.",
                    requested, bucket_label, ready_at_time
                );

                // Update the metrics to record that this message was throttled
                counters::inc_inbound_rate_limiter_throttled(bucket_label);

                // Sleep until the tokens will be available
                time_service.sleep_until(ready_at_time).await
            },
            Err(None) => {
                // The bucket will never have enough tokens
                counters::inc_inbound_rate_limiter_capacity_exceeded(bucket_label);
                return Err(PeerManagerError::RateLimitCapacityExceeded);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        protocols::{
            stream::{StreamFragment, StreamHeader},
            wire::messaging::v1::{DirectSendMsg, NetworkMessage},
        },
        ProtocolId,
    };
    use aptos_time_service::MockTimeService;
    use std::io;

    #[test]
    fn test_get_message_count_and_bytes_direct_send() {
        let msg = create_direct_send_message(42);
        let (count, bytes) = get_message_count_and_bytes(&msg);
        assert_eq!(count, 1);
        assert_eq!(bytes, 42);
    }

    #[test]
    fn test_get_message_count_and_bytes_stream_header() {
        let msg = create_stream_header(20);
        let (count, bytes) = get_message_count_and_bytes(&msg);
        assert_eq!(count, 1);
        assert_eq!(bytes, 20);
    }

    #[test]
    fn test_get_message_count_and_bytes_stream_fragment() {
        let msg = create_stream_fragment(15);
        let (count, bytes) = get_message_count_and_bytes(&msg);
        assert_eq!(count, 0);
        assert_eq!(bytes, 15);
    }

    #[test]
    fn test_get_message_count_and_bytes_error_message() {
        // Error messages should contribute 0 to both counts
        let io_err = io::Error::other("test error");
        let (count, bytes) = get_message_count_and_bytes(&Err(ReadError::IoError(io_err)));
        assert_eq!(count, 0);
        assert_eq!(bytes, 0);
    }

    #[test]
    fn test_new_returns_none_when_no_limits() {
        let time_service = TimeService::mock();
        assert!(InboundMessageRateLimiter::new(None, None, time_service).is_none());
    }

    #[test]
    fn test_new_returns_some_with_bytes_limit() {
        let time_service = TimeService::mock();
        assert!(InboundMessageRateLimiter::new(None, Some(100), time_service).is_some());
    }

    #[test]
    fn test_new_returns_some_with_messages_limit() {
        let time_service = TimeService::mock();
        assert!(InboundMessageRateLimiter::new(Some(10), None, time_service).is_some());
    }

    #[tokio::test]
    async fn test_throttle_passes_immediately_within_limit() {
        // Create an inbound message rate limiter
        let messages_per_sec = 10;
        let bytes_per_sec = 1000;
        let (mut rate_limiter, _) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // First message should pass without any waiting
        let message = create_direct_send_message(100);
        rate_limiter.throttle(&message).await.unwrap();
    }

    #[tokio::test]
    async fn test_throttle_error_exceeds_byte_capacity() {
        // Create an inbound message rate limiter
        let messages_per_sec = 50;
        let bytes_per_sec = 1;
        let (mut rate_limiter, _) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Verify that a message exceeding the byte capacity immediately errors
        let message = create_direct_send_message(100);
        let result = rate_limiter.throttle(&message).await;
        assert!(matches!(
            result,
            Err(PeerManagerError::RateLimitCapacityExceeded)
        ));
    }

    #[tokio::test]
    async fn test_throttle_error_exceeds_message_capacity() {
        // Create an inbound message rate limiter
        let messages_per_sec = 0; // Don't allow any messages
        let bytes_per_sec = 1000;
        let (mut rate_limiter, _) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Verify that any message exceeding the message capacity immediately errors
        let message = create_direct_send_message(100);
        let result = rate_limiter.throttle(&message).await;
        assert!(matches!(
            result,
            Err(PeerManagerError::RateLimitCapacityExceeded)
        ));
    }

    #[tokio::test]
    async fn test_throttle_waits_and_resumes_after_time_advance() {
        // Create an inbound message rate limiter
        let messages_per_sec = 100;
        let bytes_per_sec = 100;
        let (mut rate_limiter, mock_time) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Drain the byte bucket and then block on the next message
        let throttle_task = tokio::spawn(async move {
            rate_limiter
                .throttle(&create_direct_send_message(100))
                .await
                .unwrap();
            rate_limiter
                .throttle(&create_direct_send_message(100))
                .await
        });

        // Yield so the spawned task runs and blocks on the sleep
        tokio::task::yield_now().await;

        // Advance mock time by 1 second to trigger the refill
        mock_time.advance_secs_async(1).await;

        // The throttle should now complete successfully
        throttle_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_throttle_waits_for_message_token_refill() {
        // Create an inbound message rate limiter
        let messages_per_sec = 1;
        let bytes_per_sec = 1000;
        let (mut rate_limiter, mock_time) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Consume the message slot and then block on the next message
        let throttle_task = tokio::spawn(async move {
            rate_limiter
                .throttle(&create_direct_send_message(1))
                .await
                .unwrap();
            rate_limiter.throttle(&create_direct_send_message(1)).await
        });

        // Yield so the spawned task runs and blocks on the sleep
        tokio::task::yield_now().await;

        // Advance 1 second to refill
        mock_time.advance_secs_async(1).await;

        // Verify that the message now passes
        throttle_task.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn test_stream_fragment_does_not_count_as_message() {
        // Create an inbound message rate limiter
        let messages_per_sec = 0; // Don't allow any messages
        let bytes_per_sec = 1000;
        let (mut rate_limiter, _) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Multiple fragments should pass without consuming message tokens
        for _ in 0..10 {
            rate_limiter
                .throttle(&create_stream_fragment(0))
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn test_stream_fragment_byte_accounting() {
        // Create an inbound message rate limiter
        let messages_per_sec = 1000;
        let bytes_per_sec = 50;
        let (mut rate_limiter, mock_time) = create_rate_limiter(messages_per_sec, bytes_per_sec);

        // Drain the byte bucket and then block on the next fragment
        let throttle_task = tokio::spawn(async move {
            rate_limiter
                .throttle(&create_stream_fragment(50))
                .await
                .unwrap();
            rate_limiter.throttle(&create_stream_fragment(50)).await
        });

        // Yield so the spawned task runs and blocks on the sleep
        tokio::task::yield_now().await;

        // Advance 1 second to refill
        mock_time.advance_secs_async(1).await;

        // Verify that the message now passes
        throttle_task.await.unwrap().unwrap();
    }

    /// Creates a direct send message with the given payload size
    fn create_direct_send_message(num_bytes: usize) -> Result<MultiplexMessage, ReadError> {
        Ok(MultiplexMessage::Message(NetworkMessage::DirectSendMsg(
            DirectSendMsg {
                protocol_id: ProtocolId::ConsensusDirectSendJson,
                priority: 0,
                raw_msg: vec![0u8; num_bytes],
            },
        )))
    }

    /// Creates a stream header with the given payload size
    fn create_stream_header(num_bytes: usize) -> Result<MultiplexMessage, ReadError> {
        let message = NetworkMessage::DirectSendMsg(DirectSendMsg {
            protocol_id: ProtocolId::ConsensusDirectSendJson,
            priority: 0,
            raw_msg: vec![0u8; num_bytes],
        });
        Ok(MultiplexMessage::Stream(StreamMessage::Header(
            StreamHeader {
                request_id: 0,
                num_fragments: 1,
                message,
            },
        )))
    }

    /// Creates a stream fragment with the given payload size
    fn create_stream_fragment(num_bytes: usize) -> Result<MultiplexMessage, ReadError> {
        Ok(MultiplexMessage::Stream(StreamMessage::Fragment(
            StreamFragment {
                request_id: 0,
                fragment_id: 0,
                raw_data: vec![0u8; num_bytes],
            },
        )))
    }

    /// Creates a rate limiter with a mock time service
    fn create_rate_limiter(
        messages_per_sec: u64,
        bytes_per_sec: u64,
    ) -> (InboundMessageRateLimiter, MockTimeService) {
        let mock_time_service = MockTimeService::new();
        let time_service = TimeService::from_mock(mock_time_service.clone());
        let rate_limiter = InboundMessageRateLimiter::new(
            Some(messages_per_sec),
            Some(bytes_per_sec),
            time_service,
        )
        .unwrap();
        (rate_limiter, mock_time_service)
    }
}
