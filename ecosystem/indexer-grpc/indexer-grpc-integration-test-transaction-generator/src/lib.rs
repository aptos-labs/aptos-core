// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod capture_transactions;
pub mod move_executor;
pub mod transaction_generator;

// Move fixtures directory name.
pub const MOVE_FIXTURES_DIR: &str = "move_fixtures";
pub const APTOS_CLI_BINARY_NAME: &str = "aptos";
// TODO: support local faucet url in config.
pub const DEFAULT_LOCAL_NODE_API_URL: &str = "http://localhost:8080";
pub const DEFAULT_LOCAL_FAUCET_URL: &str = "http://localhost:8081";
