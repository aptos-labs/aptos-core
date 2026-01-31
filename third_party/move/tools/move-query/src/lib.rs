// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Move Query Tool
//!
//! Provides programmatic access to the Move model (GlobalEnv) built by the compiler.
//! Supports both direct API access and MCP server mode for AI agent integration.
//!
//! # Architecture
//!
//! The library is organized into:
//! - `types`: Data structures representing Move package elements (modules, functions, structs)
//! - `engine`: QueryEngine that wraps GlobalEnv and provides query methods
//! - `mcp`: Model Context Protocol server for AI agent integration
//!
//! # Example Usage
//!
//! ```ignore
//! use move_query::{QueryEngine, QueryOptions};
//! use std::path::PathBuf;
//!
//! let options = QueryOptions::default();
//! let engine = QueryEngine::new(PathBuf::from("./my-package"), options)?;
//!
//! // Get package overview
//! let package = engine.get_package()?;
//!
//! // Get module details
//! let module = engine.get_module("my_module")?;
//!
//! // Search for functions
//! let results = engine.search("transfer", Some(SearchLevel::Function), 10)?;
//! ```

pub mod engine;
pub mod mcp;
pub mod types;

pub use engine::{QueryEngine, QueryError, QueryOptions, QueryResult};
pub use types::*;
