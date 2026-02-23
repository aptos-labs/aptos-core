// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Proxy Primary Consensus
//!
//! This crate implements proxy primary consensus where a subset of validators (proxies)
//! run a second RoundManager instance (standard Aptos BFT) with proxy hooks and forward
//! ordered blocks to the full validator set (primaries).
//!
//! # Architecture
//!
//! - **ProxyEvents**: Communication channel types between primary and proxy RoundManagers
//! - **PrimaryIntegration**: Aggregates proxy blocks into primary blocks
//!
//! Safety rules for proxy consensus use a standard `SafetyRules` instance from the
//! `aptos-safety-rules` crate with separate in-memory storage (independent voting state).

#![forbid(unsafe_code)]

pub mod primary_integration;
pub mod proxy_error;
pub mod proxy_events;
pub mod proxy_metrics;

pub use primary_integration::PrimaryBlockFromProxy;
pub use proxy_error::ProxyConsensusError;
pub use proxy_events::{
    AtomicPipelineState, PipelineBackpressureInfo, PrimaryToProxyEvent, ProxyToPrimaryEvent,
};
