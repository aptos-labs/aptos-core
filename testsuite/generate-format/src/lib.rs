// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! How and where to record the Serde format of interesting Aptos types.
//! See API documentation with `cargo doc -p serde-reflection --open`

use clap::{Parser, ValueEnum};
use serde_reflection::Registry;
use std::fmt::{Display, Formatter};

/// Rest API types
mod api;
/// Aptos transactions.
mod aptos;
/// Consensus messages.
mod consensus;
/// Analyze Serde formats to detect certain patterns.
mod linter;
/// Move ABI.
mod move_abi;
/// Network messages.
mod network;

pub use linter::lint_bcs_format;

#[derive(Debug, Parser, Clone, Copy, ValueEnum)]
/// A corpus of Rust types to trace, and optionally record on disk.
pub enum Corpus {
    API,
    Aptos,
    Consensus,
    Network,
    MoveABI,
}

impl Corpus {
    /// Compute the registry of formats.
    pub fn get_registry(self) -> Registry {
        let result = match self {
            Corpus::API => api::get_registry(),
            Corpus::Aptos => aptos::get_registry(),
            Corpus::Consensus => consensus::get_registry(),
            Corpus::Network => network::get_registry(),
            Corpus::MoveABI => move_abi::get_registry(),
        };
        match result {
            Ok(registry) => registry,
            Err(error) => {
                panic!("{}:{}", error, error.explanation());
            },
        }
    }

    /// Where to record this corpus on disk.
    pub fn output_file(self) -> Option<&'static str> {
        match self {
            Corpus::API => api::output_file(),
            Corpus::Aptos => aptos::output_file(),
            Corpus::Consensus => consensus::output_file(),
            Corpus::Network => network::output_file(),
            Corpus::MoveABI => move_abi::output_file(),
        }
    }
}

impl Display for Corpus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Corpus::API => "API",
            Corpus::Aptos => "Aptos",
            Corpus::Consensus => "Consensus",
            Corpus::Network => "Network",
            Corpus::MoveABI => "MoveABI",
        })
    }
}
