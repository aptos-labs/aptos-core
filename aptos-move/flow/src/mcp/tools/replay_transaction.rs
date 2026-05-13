// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Replay a committed on-chain Aptos transaction locally, optionally with
//! local Move package module overrides.

use super::super::session::FlowSession;
use rmcp::{handler::server::router::tool::ToolRouter, tool_router};

#[tool_router(router = replay_transaction_router, vis = "pub(crate)")]
impl FlowSession {}
