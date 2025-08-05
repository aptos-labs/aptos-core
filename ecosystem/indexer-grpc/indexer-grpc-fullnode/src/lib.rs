// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_api::context::Context;
use std::sync::Arc;

pub mod convert;
pub mod counters;
pub mod fullnode_data_service;
pub mod localnet_data_service;
pub mod localnet_websocket_service;
pub mod runtime;
pub mod stream_coordinator;

#[derive(Clone, Debug)]
pub struct ServiceContext {
    pub context: Arc<Context>,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
}

#[cfg(test)]
pub(crate) mod tests;
