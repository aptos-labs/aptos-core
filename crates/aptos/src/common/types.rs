// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{fmt::Debug, str::FromStr};
use structopt::StructOpt;
use thiserror::Error;

/// A common result to be returned to users
pub type CliResult = Result<String, String>;

/// TODO: Re-evaluate these errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid key value found in backend: {0}")]
    BackendInvalidKeyValue(String),
    #[error("Backend is missing the backend key")]
    BackendMissingBackendKey,
    #[error("Backend parsing error: {0}")]
    BackendParsingError(String),
    #[error("Invalid arguments: {0}")]
    CommandArgumentError(String),
    #[error("Unable to load config: {0}")]
    ConfigError(String),
    #[error("Error accessing '{0}': {1}")]
    IO(String, #[source] std::io::Error),
    #[error("Error (de)serializing '{0}': {1}")]
    BCS(String, #[source] bcs::Error),
    #[error("Unable to decode network address: {0}")]
    NetworkAddressDecodeError(String),
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    #[error("Unable to parse file '{0}', error: {1}")]
    UnableToParseFile(String, String),
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    #[error("Unexpected command, expected {0}, found {1}")]
    UnexpectedCommand(String, String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    #[error("Aborted command")]
    AbortedError,
}

/// Types of Keys used by the blockchain
#[derive(Clone, Copy, Debug, StructOpt)]
pub enum KeyType {
    Ed25519,
    X25519,
}

impl FromStr for KeyType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ed25519" => Ok(KeyType::Ed25519),
            "x25519" => Ok(KeyType::X25519),
            _ => Err("Invalid key type"),
        }
    }
}

/// Types of encodings used by the blockchain
#[derive(Clone, Copy, Debug, StructOpt)]
pub enum EncodingType {
    /// Binary Canonical Serialization
    BCS,
    /// Hex encoded e.g. 0xABCDE12345
    Hex,
    /// Base 64 encoded
    Base64,
}

impl FromStr for EncodingType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hex" => Ok(EncodingType::Hex),
            "bcs" => Ok(EncodingType::BCS),
            "base64" => Ok(EncodingType::Base64),
            _ => Err("Invalid encoding type"),
        }
    }
}

/// An insertable option for use with prompts.
#[derive(Debug, StructOpt)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[structopt(long)]
    pub assume_yes: bool,
}

/// An insertable option for use with encodings.
#[derive(Debug, StructOpt)]
pub struct EncodingOptions {
    /// Encoding of data as `base64`, `bcs`, or `hex`
    #[structopt(long, default_value = "hex")]
    pub encoding: EncodingType,
}
