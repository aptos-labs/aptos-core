// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-block consensus futures. The legacy phase machinery is retained for DAG and observers.
mod commit_certificate;
pub mod commit_reliable_broadcast;
pub mod commit_signer;
pub mod decryption_pipeline_builder;
pub mod errors;
pub mod execution_client;
pub mod pipeline_builder;
#[cfg(test)]
mod tests;

// Legacy DAG and observer execution path.
pub mod buffer;
pub mod buffer_item;
pub mod buffer_manager;
pub mod commit_pipeline_futures;
pub mod decoupled_execution_utils;
pub mod execution_schedule_phase;
pub mod execution_wait_phase;
pub mod hashable;
pub mod persisting_phase;
pub mod pipeline_phase;
pub mod signing_phase;
