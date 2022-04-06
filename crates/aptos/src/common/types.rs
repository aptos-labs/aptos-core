// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::op::key::{decode_key, load_key};
use aptos_crypto::x25519;
use clap::Parser;
use std::{fmt::Debug, path::PathBuf, str::FromStr};
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
#[derive(Clone, Copy, Debug, Parser)]
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
#[derive(Clone, Copy, Debug, Parser)]
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
#[derive(Debug, Parser)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[clap(long)]
    pub assume_yes: bool,
}

/// An insertable option for use with encodings.
#[derive(Debug, Parser)]
pub struct EncodingOptions {
    /// Encoding of data as `base64`, `bcs`, or `hex`
    #[clap(long, default_value = "hex")]
    pub encoding: EncodingType,
}

#[derive(Debug, Parser)]
pub struct PrivateKeyInputOptions {
    /// Private key input file name
    #[clap(long, group = "key_input", parse(from_os_str))]
    private_key_file: Option<PathBuf>,
    /// Private key encoded in a type as shown in `encoding`
    #[clap(long, group = "key_input")]
    private_key: Option<String>,
}

impl PrivateKeyInputOptions {
    pub fn extract_private_key(&self, encoding: EncodingType) -> Result<x25519::PrivateKey, Error> {
        if let Some(ref file) = self.private_key_file {
            load_key(file.as_path(), encoding)
        } else if let Some(ref key) = self.private_key {
            let key = key.as_bytes().to_vec();
            decode_key(key, encoding)
        } else {
            Err(Error::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'] must be used".to_string(),
            ))
        }
    }
}

#[derive(Debug, Parser)]
pub struct PublicKeyInputOptions {
    /// Public key input file name.
    #[clap(long, group = "key_input", parse(from_os_str))]
    public_key_file: Option<PathBuf>,
    /// Public key encoded in a type as shown in `encoding`
    #[clap(long, group = "key_input")]
    public_key: Option<String>,
}

impl PublicKeyInputOptions {
    pub fn extract_public_key(&self, encoding: EncodingType) -> Result<x25519::PublicKey, Error> {
        if let Some(ref file) = self.public_key_file {
            load_key(file.as_path(), encoding)
        } else if let Some(ref key) = self.public_key {
            let key = key.as_bytes().to_vec();
            decode_key(key, encoding)
        } else {
            Err(Error::CommandArgumentError(
                "One of ['--public-key', '--public-key-file'] must be used".to_string(),
            ))
        }
    }
}

#[derive(Debug, Parser)]
pub struct KeyInputOptions {
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    public_key_options: PublicKeyInputOptions,
}

impl KeyInputOptions {
    pub fn extract_public_key(&self, encoding: EncodingType) -> Result<x25519::PublicKey, Error> {
        let private_key_result = self.private_key_options.extract_private_key(encoding);
        let public_key_result = self.public_key_options.extract_public_key(encoding);

        if let Ok(private_key) = private_key_result {
            Ok(private_key.public_key())
        } else if let Ok(public_key) = public_key_result {
            Ok(public_key)
        } else {
            // TODO: merge above errors better
            Err(Error::CommandArgumentError("One of ['--private-key', '--private-key-file', '--public-key', '--public-key-file'] must be used".to_string()))
        }
    }
}
