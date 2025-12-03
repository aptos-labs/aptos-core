// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

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
