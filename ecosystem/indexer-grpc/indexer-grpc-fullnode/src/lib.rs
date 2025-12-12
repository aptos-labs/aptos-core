// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_api::context::Context;
use std::sync::Arc;

pub mod convert;
pub mod counters;
pub mod fullnode_data_service;
pub mod localnet_data_service;
pub mod runtime;
pub mod stream_coordinator;

#[derive(Clone, Debug)]
pub struct ServiceContext {
    pub context: Arc<Context>,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
    pub transaction_channel_size: usize,
    pub max_transaction_filter_size_bytes: usize,
}

#[cfg(test)]
pub(crate) mod tests;
