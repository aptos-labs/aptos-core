// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{MultiplexMessage, MultiplexMessageStream, ReadError};
use crate::{counters, protocols::stream::StreamMessage};
use aptos_config::network_id::NetworkContext;
use aptos_token_bucket::SharedTokenBucket;
use futures::{io::AsyncRead, ready, stream::Stream};
use pin_project::pin_project;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::time::{sleep_until, Instant, Sleep};

// Conservative estimate of serialization overhead per message in bytes
const MESSAGE_SERIALIZATION_OVERHEAD_BYTES: usize = 150;

/// A rate-limited wrapper around MultiplexMessageStream that enforces per-connection
/// inbound rate limiting for both bytes/second and messages/second.
/// The stream applies backpressure by returning Poll::Pending when rate limits are
/// exceeded, which causes the TCP socket buffer to fill and triggers TCP flow control.
#[pin_project]
pub struct RateLimitedMultiplexMessageStream<TReadSocket: AsyncRead + Unpin> {
    /// Network context for logging and metrics
    network_context: NetworkContext,

    /// The underlying stream being wrapped
    #[pin]
    inner: MultiplexMessageStream<TReadSocket>,

    /// Rate limiter for bytes per second (if None, no rate limiting)
    byte_limiter: Option<SharedTokenBucket>,

    /// Rate limiter for messages per second (if None, no rate limiting)
    message_limiter: Option<SharedTokenBucket>,

    /// Sleep future for when we're rate limited.
    /// Note: We need to pin this because Sleep is !Unpin.
    delay: Option<Pin<Box<Sleep>>>,
}

impl<TReadSocket: AsyncRead + Unpin> RateLimitedMultiplexMessageStream<TReadSocket> {
    pub fn new(
        network_context: NetworkContext,
        inner: MultiplexMessageStream<TReadSocket>,
        byte_limiter: Option<SharedTokenBucket>,
        message_limiter: Option<SharedTokenBucket>,
    ) -> Self {
        Self {
            network_context,
            inner,
            byte_limiter,
            message_limiter,
            delay: None,
        }
    }
}

impl<TReadSocket: AsyncRead + Unpin> Stream for RateLimitedMultiplexMessageStream<TReadSocket> {
    type Item = Result<MultiplexMessage, ReadError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // If we're waiting due to rate limiting, poll the delay first
        if let Some(delay) = this.delay {
            ready!(delay.as_mut().poll(cx));
            *this.delay = None;
        }

        // Poll the inner stream for the next message
        let maybe_message = ready!(this.inner.as_mut().poll_next(cx));
        let multiplex_message = match maybe_message {
            Some(Ok(multiplex_message)) => multiplex_message,
            Some(Err(error)) => return Poll::Ready(Some(Err(error))),
            None => return Poll::Ready(None),
        };

        // Calculate the size of the message in bytes
        let message_bytes = calculate_message_bytes(&multiplex_message);

        // Try to acquire byte tokens first
        if let Some(byte_limiter) = this.byte_limiter {
            let mut bucket = byte_limiter.lock();
            match bucket.try_acquire_all(message_bytes) {
                Ok(()) => {
                    // Tokens acquired (byte limit passed). Update the metrics.
                    counters::update_inbound_byte_rate_limit_tokens(
                        this.network_context,
                        counters::TOKENS_GRANTED_LABEL,
                        message_bytes,
                    );
                },
                Err(maybe_wait_time) => {
                    // Tokens not acquired (rate limited). Update the metrics.
                    counters::update_inbound_byte_rate_limit_tokens(
                        this.network_context,
                        counters::TOKENS_DENIED_LABEL,
                        message_bytes,
                    );

                    // Process the error
                    return match maybe_wait_time {
                        None => {
                            // The message is too large for the bucket (reject with an error!)
                            Poll::Ready(Some(Err(ReadError::IoError(std::io::Error::other(
                                format!(
                                    "Message size {} exceeds rate limit bucket capacity!",
                                    message_bytes
                                ),
                            )))))
                        },
                        Some(wait_time) => {
                            // Set the delay and return pending to apply backpressure
                            *this.delay = Some(Box::pin(sleep_until(Instant::from_std(wait_time))));
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        },
                    };
                },
            }
        }

        // Next, try to acquire message tokens
        if let Some(message_limiter) = this.message_limiter {
            let mut bucket = message_limiter.lock();
            match bucket.try_acquire_all(1) {
                Ok(()) => {
                    // Tokens acquired (message limit passed). Update the metrics.
                    counters::update_inbound_message_rate_limit_tokens(
                        this.network_context,
                        counters::TOKENS_GRANTED_LABEL,
                        1,
                    );
                },
                Err(maybe_wait_time) => {
                    // Tokens not acquired (rate limited). Update the metrics.
                    counters::update_inbound_message_rate_limit_tokens(
                        this.network_context,
                        counters::TOKENS_DENIED_LABEL,
                        1,
                    );

                    // Return the byte tokens since we're not able to process the message yet
                    if let Some(byte_limiter) = this.byte_limiter {
                        byte_limiter.lock().return_tokens(message_bytes);
                    }

                    if let Some(wait_time) = maybe_wait_time {
                        // Set the delay and return pending to apply backpressure
                        *this.delay = Some(Box::pin(sleep_until(Instant::from_std(wait_time))));
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    } else {
                        // This should not happen since we only requested 1 token
                        return Poll::Ready(Some(Err(ReadError::IoError(std::io::Error::other(
                            "Failed to acquire a single message token from the message bucket!",
                        )))));
                    }
                },
            }
        }

        // Both limits passed (return the message)
        Poll::Ready(Some(Ok(multiplex_message)))
    }
}

/// Calculates the size of a MultiplexMessage in bytes.
/// Note: we use conservative estimates here since we don't have exact serialization size.
fn calculate_message_bytes(message: &MultiplexMessage) -> u64 {
    let message_bytes = match message {
        MultiplexMessage::Message(network_message) => {
            network_message.data_len() + MESSAGE_SERIALIZATION_OVERHEAD_BYTES
        },
        MultiplexMessage::Stream(stream_msg) => match stream_msg {
            StreamMessage::Header(header) => {
                header.message.data_len() + MESSAGE_SERIALIZATION_OVERHEAD_BYTES
            },
            StreamMessage::Fragment(fragment) => {
                fragment.raw_data.len() + MESSAGE_SERIALIZATION_OVERHEAD_BYTES
            },
        },
    };

    message_bytes as u64
}

#[cfg(test)]
mod tests {
    // TODO: Add unit tests for rate limiting behavior
    // These would require mocking the MultiplexMessageStream
}
