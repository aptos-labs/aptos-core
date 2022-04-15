// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::utils::{check_if_file_exists, write_to_file};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    x25519, PrivateKey, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use aptos_logger::{debug, info};
use clap::{ArgEnum, Parser};
use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    path::{Path, PathBuf},
    str::FromStr,
};
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
    #[error("Unable to find config {0}, have you run `aptos init`?")]
    ConfigNotFoundError(String),
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
    #[error("Move compiliation failed: {0}")]
    MoveCompiliationError(String),
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CliConfig {
    /// Private key for commands.  TODO: Add vault functionality
    pub private_key: Option<Ed25519PrivateKey>,
}

impl CliConfig {
    /// Checks if the config exists in the current working directory
    pub fn config_exists() -> Result<bool, Error> {
        Self::aptos_folder().map(|folder| folder.exists())
    }

    /// Loads the config from the current working directory
    pub fn load() -> Result<Self, Error> {
        let config_file = Self::aptos_folder()?.join("config.yml");
        if !config_file.exists() {
            return Err(Error::ConfigError(format!("{:?}", config_file)));
        }

        let bytes = std::fs::read(&config_file)
            .map_err(|err| Error::IO(format!("Failed to read {:?}", config_file), err))?;
        serde_yaml::from_slice(&bytes)
            .map_err(|err| Error::UnableToParseFile(format!("{:?}", config_file), err.to_string()))
    }

    /// Saves the config to ./.aptos/config.yml
    pub fn save(&self) -> Result<(), Error> {
        let aptos_folder = Self::aptos_folder()?;

        // Create if it doesn't exist
        if !aptos_folder.exists() {
            std::fs::create_dir(&aptos_folder).map_err(|err| {
                Error::CommandArgumentError(format!(
                    "Unable to create {:?} directory {}",
                    aptos_folder, err
                ))
            })?;
            info!("Created .aptos/ folder");
        } else {
            debug!(".aptos/ folder already initialized");
        }

        // Save over previous config file
        // TODO: Ask for saving over?
        let config_file = aptos_folder.join("config.yml");
        let config_bytes = serde_yaml::to_string(&self)
            .map_err(|err| Error::UnexpectedError(format!("Failed to serialize config {}", err)))?;
        write_to_file(&config_file, "config.yml", config_bytes.as_bytes())?;
        Ok(())
    }

    /// Finds the current directory's .aptos folder
    fn aptos_folder() -> Result<PathBuf, Error> {
        std::env::current_dir()
            .map_err(|err| {
                Error::UnexpectedError(format!("Unable to get current directory {}", err))
            })
            .map(|dir| dir.join(".aptos"))
    }
}

/// Types of Keys used by the blockchain
#[derive(ArgEnum, Clone, Copy, Debug)]
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
#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum EncodingType {
    /// Binary Canonical Serialization
    BCS,
    /// Hex encoded e.g. 0xABCDE12345
    Hex,
    /// Base 64 encoded
    Base64,
}

impl EncodingType {
    /// Encodes `Key` into one of the `EncodingType`s
    pub fn encode_key<Key: ValidCryptoMaterial>(
        &self,
        key: &Key,
        key_name: &str,
    ) -> Result<Vec<u8>, Error> {
        Ok(match self {
            EncodingType::Hex => hex::encode_upper(key.to_bytes()).into_bytes(),
            EncodingType::BCS => {
                bcs::to_bytes(key).map_err(|err| Error::BCS(key_name.to_string(), err))?
            }
            EncodingType::Base64 => base64::encode(key.to_bytes()).into_bytes(),
        })
    }

    /// Loads a key from a file
    pub fn load_key<Key: ValidCryptoMaterial>(&self, path: &Path) -> Result<Key, Error> {
        let data = std::fs::read(&path).map_err(|err| {
            Error::UnableToReadFile(path.to_str().unwrap().to_string(), err.to_string())
        })?;

        self.decode_key(data)
    }

    /// Decodes an encoded key given the known encoding
    pub fn decode_key<Key: ValidCryptoMaterial>(&self, data: Vec<u8>) -> Result<Key, Error> {
        match self {
            EncodingType::BCS => {
                bcs::from_bytes(&data).map_err(|err| Error::BCS("Key".to_string(), err))
            }
            EncodingType::Hex => {
                let hex_string = String::from_utf8(data).unwrap();
                Key::from_encoded_string(hex_string.trim())
                    .map_err(|err| Error::UnableToParse("Key", err.to_string()))
            }
            EncodingType::Base64 => {
                let string = String::from_utf8(data).unwrap();
                let bytes = base64::decode(string.trim())
                    .map_err(|err| Error::UnableToParse("Key", err.to_string()))?;
                Key::try_from(bytes.as_slice())
                    .map_err(|err| Error::UnexpectedError(format!("Failed to parse key {}", err)))
            }
        }
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
    pub fn extract_private_key(
        &self,
        encoding: EncodingType,
    ) -> Result<Option<Ed25519PrivateKey>, Error> {
        if let Some(ref file) = self.private_key_file {
            encoding.load_key(file.as_path()).map(Some)
        } else if let Some(ref key) = self.private_key {
            let key = key.as_bytes().to_vec();
            encoding.decode_key(key).map(Some)
        } else {
            Ok(None)
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
    pub fn extract_public_key(
        &self,
        encoding: EncodingType,
    ) -> Result<Option<Ed25519PublicKey>, Error> {
        if let Some(ref file) = self.public_key_file {
            encoding.load_key(file.as_path()).map(Some)
        } else if let Some(ref key) = self.public_key {
            let key = key.as_bytes().to_vec();
            encoding.decode_key(key).map(Some)
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Parser)]
pub struct KeyInputOptions {
    #[clap(flatten)]
    pub private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    public_key_options: PublicKeyInputOptions,
}

impl KeyInputOptions {
    /// Extracts public key from either private or public key options
    pub fn extract_public_key(&self, encoding: EncodingType) -> Result<Ed25519PublicKey, Error> {
        let private_key_result = self.private_key_options.extract_private_key(encoding)?;
        let public_key_result = self.public_key_options.extract_public_key(encoding)?;

        if let Some(private_key) = private_key_result {
            Ok(private_key.public_key())
        } else if let Some(public_key) = public_key_result {
            Ok(public_key)
        } else {
            let config = CliConfig::load()?;
            if let Some(private_key) = config.private_key {
                println!("Using .aptos/config.yml private key");
                Ok(private_key.public_key())
            } else {
                Err(Error::CommandArgumentError("One of ['--private-key', '--private-key-file', '--public-key', '--public-key-file'] must be used".to_string()))
            }
        }
    }

    pub fn extract_x25519_public_key(
        &self,
        encoding: EncodingType,
    ) -> Result<x25519::PublicKey, Error> {
        let key = self.extract_public_key(encoding)?;
        x25519::PublicKey::from_ed25519_public_bytes(&key.to_bytes()).map_err(|err| {
            Error::UnexpectedError(format!("Failed to convert ed25519 to x25519 {:?}", err))
        })
    }
}

#[derive(Debug, Parser)]
pub struct SaveFile {
    /// Output file name
    #[clap(long, parse(from_os_str))]
    pub output_file: PathBuf,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl SaveFile {
    /// Check if the key file exists already
    pub fn check_file(&self) -> Result<(), Error> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options.assume_yes)
    }

    /// Save to the `output_file`
    pub fn save_to_file(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        write_to_file(self.output_file.as_path(), name, bytes)
    }
}

#[derive(Debug, Parser)]
pub struct NodeOptions {
    #[clap(
        long,
        parse(try_from_str),
        default_value = "https://fullnode.devnet.aptoslabs.com"
    )]
    pub url: reqwest::Url,
}

/// Options for a move package dir
#[derive(Debug, Parser)]
pub struct MovePackageDir {
    /// Path to a move package (the folder with a Move.toml file)
    #[clap(long, parse(from_os_str))]
    pub package_dir: PathBuf,
    /// Path to save the compiled move package
    #[clap(long, parse(from_os_str))]
    pub output_dir: Option<PathBuf>,
}
