// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![allow(dead_code)]

mod data_notification;
mod data_stream;
mod error;
mod logging;
mod stream_engine;
mod streaming_client;
mod streaming_service;

#[cfg(test)]
mod tests;
