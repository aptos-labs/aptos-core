// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! MCP (Model Context Protocol) server for AI agent integration
//!
//! This module implements a JSON-RPC based server that exposes Move package
//! query capabilities via the Model Context Protocol.

mod server;

pub use server::run_server;
