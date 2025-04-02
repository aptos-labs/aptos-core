// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides utility for reading and writing crypto keys
//! in different formats used by the blockchain.

use crate::{traits::ValidCryptoMaterialStringExt, ValidCryptoMaterial};
use core::{
    fmt::{Display, Formatter},
    str::FromStr,
};
use std::{fmt::Debug, path::Path};
use thiserror::Error;

/// Encoding error
#[derive(Debug, Error)]
pub enum EncodingError {
    /// Error encoding or decoding BCS
    #[error("Error (de)serializing '{0}': {1}")]
    BCS(&'static str, bcs::Error),
    /// Error for unable to parse
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    /// Error for unable to read a given file
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    /// UTF8 error
    #[error("Unexpected error: {0}")]
    UTF8(String),
}

impl std::convert::From<std::string::FromUtf8Error> for EncodingError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        EncodingError::UTF8(e.to_string())
    }
}

/// Types of encodings used by the blockchain
#[derive(Clone, Copy, Debug, Default)]
pub enum EncodingType {
    /// Binary Canonical Serialization
    BCS,
    /// Hex encoded e.g. 0xABCDE12345
    #[default]
    Hex,
    /// Base 64 encoded
    Base64,
}

impl EncodingType {
    /// Encodes `Key` into one of the `EncodingType`s
    pub fn encode_key<Key: ValidCryptoMaterial>(
        &self,
        name: &'static str,
        key: &Key,
    ) -> Result<Vec<u8>, EncodingError> {
        Ok(match self {
            EncodingType::Hex => hex::encode_upper(key.to_bytes()).into_bytes(),
            EncodingType::BCS => bcs::to_bytes(key).map_err(|err| EncodingError::BCS(name, err))?,
            EncodingType::Base64 => base64::encode(key.to_bytes()).into_bytes(),
        })
    }

    /// Loads a key from a file
    pub fn load_key<Key: ValidCryptoMaterial>(
        &self,
        name: &'static str,
        path: &Path,
    ) -> Result<Key, EncodingError> {
        self.decode_key(name, read_from_file(path)?)
    }

    /// Decodes an encoded key given the known encoding
    pub fn decode_key<Key: ValidCryptoMaterial>(
        &self,
        name: &'static str,
        data: Vec<u8>,
    ) -> Result<Key, EncodingError> {
        match self {
            EncodingType::BCS => {
                bcs::from_bytes(&data).map_err(|err| EncodingError::BCS(name, err))
            },
            EncodingType::Hex => {
                let hex_string = String::from_utf8(data)?;
                Key::from_encoded_string(hex_string.trim())
                    .map_err(|err| EncodingError::UnableToParse(name, err.to_string()))
            },
            EncodingType::Base64 => {
                let string = String::from_utf8(data)?;
                let bytes = base64::decode(string.trim())
                    .map_err(|err| EncodingError::UnableToParse(name, err.to_string()))?;
                Key::try_from(bytes.as_slice()).map_err(|err| {
                    EncodingError::UnableToParse(name, format!("Failed to parse key {:?}", err))
                })
            },
        }
    }
}

/// Reads bytes from files
///
/// TODO: verify that this isn't duplicated
pub fn read_from_file(path: &Path) -> Result<Vec<u8>, EncodingError> {
    std::fs::read(path)
        .map_err(|e| EncodingError::UnableToReadFile(format!("{}", path.display()), e.to_string()))
}

impl Display for EncodingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            EncodingType::BCS => "bcs",
            EncodingType::Hex => "hex",
            EncodingType::Base64 => "base64",
        };
        write!(f, "{}", str)
    }
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
