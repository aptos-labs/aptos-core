// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Protocols used by network module for external APIs and internal functionality
//!
//! Each protocol corresponds to a certain order of messages
pub mod direct_send;
pub mod health_checker;
pub mod identity;
pub mod network;
pub mod rpc;
pub mod stream;
pub mod wire;
