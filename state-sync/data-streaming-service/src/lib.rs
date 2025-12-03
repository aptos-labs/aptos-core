#![forbid(unsafe_code)] // Copyright (c) Aptos Foundation
                        // Copyright (c) Aptos Foundation
                        // SPDX-License-Identifier: Innovation-Enabling Source Code License

// SPDX-License-Identifier: Innovation-Enabling Source Code License

pub mod data_notification;
pub mod data_stream;
mod dynamic_prefetching;
pub mod error;
mod logging;
mod metrics;
mod stream_engine;
pub mod streaming_client;
pub mod streaming_service;

#[cfg(test)]
mod tests;
