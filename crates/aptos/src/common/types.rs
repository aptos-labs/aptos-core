// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::utils::prompt_yes_with_override;
use crate::{
    common::{
        init::{DEFAULT_FAUCET_URL, DEFAULT_REST_URL},
        utils::{
            chain_id, check_if_file_exists, create_dir_if_not_exist, dir_default_to_current,
            get_auth_key, get_sequence_number, read_from_file, start_logger, to_common_result,
            to_common_success_result, write_to_file, write_to_file_with_opts,
            write_to_user_only_file,
        },
    },
    config::GlobalConfig,
    genesis::git::from_yaml,
};
use aptos_crypto::ed25519::Ed25519Signature;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    x25519, PrivateKey, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use aptos_global_constants::adjust_gas_headroom;
use aptos_keygen::KeyGen;
use aptos_rest_client::aptos_api_types::{ExplainVMStatus, HashValue, UserTransaction};
use aptos_rest_client::error::RestError;
use aptos_rest_client::{Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::transaction::{
    authenticator::AuthenticationKey, SignedTransaction, TransactionPayload,
};
use async_trait::async_trait;
use clap::{ArgEnum, Parser};
use hex::FromHexError;
use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    fmt::{Debug, Display, Formatter},
    fs::OpenOptions,
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};
use thiserror::Error;

const MAX_POSSIBLE_GAS_UNITS: u64 = 1_000_000;

/// A common result to be returned to users
pub type CliResult = Result<String, String>;

/// A common result to remove need for typing `Result<T, CliError>`
pub type CliTypedResult<T> = Result<T, CliError>;

/// CLI Errors for reporting through telemetry and outputs
#[derive(Debug, Error)]
pub enum CliError {
    #[error("Aborted command")]
    AbortedError,
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Error (de)serializing '{0}': {1}")]
    BCS(&'static str, #[source] bcs::Error),
    #[error("Invalid arguments: {0}")]
    CommandArgumentError(String),
    #[error("Unable to load config: {0} {1}")]
    ConfigLoadError(String, String),
    #[error("Unable to find config {0}, have you run `aptos init`?")]
    ConfigNotFoundError(String),
    #[error("Error accessing '{0}': {1}")]
    IO(String, #[source] std::io::Error),
    #[error("Move compilation failed: {0}")]
    MoveCompilationError(String),
    #[error("Move unit tests failed")]
    MoveTestError,
    #[error("Move Prover failed: {0}")]
    MoveProverError(String),
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    #[error("Simulation failed with status: {0}")]
    SimulationError(String),
}

impl CliError {
    pub fn to_str(&self) -> &'static str {
        match self {
            CliError::AbortedError => "AbortedError",
            CliError::ApiError(_) => "ApiError",
            CliError::BCS(_, _) => "BCS",
            CliError::CommandArgumentError(_) => "CommandArgumentError",
            CliError::ConfigLoadError(_, _) => "ConfigLoadError",
            CliError::ConfigNotFoundError(_) => "ConfigNotFoundError",
            CliError::IO(_, _) => "IO",
            CliError::MoveCompilationError(_) => "MoveCompilationError",
            CliError::MoveTestError => "MoveTestError",
            CliError::MoveProverError(_) => "MoveProverError",
            CliError::UnableToParse(_, _) => "UnableToParse",
            CliError::UnableToReadFile(_, _) => "UnableToReadFile",
            CliError::UnexpectedError(_) => "UnexpectedError",
            CliError::SimulationError(_) => "SimulationError",
        }
    }
}

impl From<RestError> for CliError {
    fn from(e: RestError) -> Self {
        CliError::ApiError(e.to_string())
    }
}

impl From<aptos_config::config::Error> for CliError {
    fn from(e: aptos_config::config::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_github_client::Error> for CliError {
    fn from(e: aptos_github_client::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(e: serde_yaml::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<base64::DecodeError> for CliError {
    fn from(e: base64::DecodeError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<std::string::FromUtf8Error> for CliError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_crypto::CryptoMaterialError> for CliError {
    fn from(e: aptos_crypto::CryptoMaterialError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<hex::FromHexError> for CliError {
    fn from(e: FromHexError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<anyhow::Error> for CliError {
    fn from(e: anyhow::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<bcs::Error> for CliError {
    fn from(e: bcs::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

/// Config saved to `.aptos/config.yaml`
#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    /// Map of profile configs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<BTreeMap<String, ProfileConfig>>,
}

const CONFIG_FILE: &str = "config.yaml";
const LEGACY_CONFIG_FILE: &str = "config.yml";
pub const CONFIG_FOLDER: &str = ".aptos";

/// An individual profile
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Private key for commands.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<Ed25519PrivateKey>,
    /// Public key for commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<Ed25519PublicKey>,
    /// Account for commands
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AccountAddress>,
    /// URL for the Aptos rest endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    /// URL for the Faucet endpoint (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
}

/// ProfileConfig but without the private parts
#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub has_private_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<Ed25519PublicKey>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
}

impl From<&ProfileConfig> for ProfileSummary {
    fn from(config: &ProfileConfig) -> Self {
        ProfileSummary {
            has_private_key: config.private_key.is_some(),
            public_key: config.public_key.clone(),
            account: config.account,
            rest_url: config.rest_url.clone(),
            faucet_url: config.faucet_url.clone(),
        }
    }
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            profiles: Some(BTreeMap::new()),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum ConfigSearchMode {
    CurrentDir,
    CurrentDirAndParents,
}

impl CliConfig {
    /// Checks if the config exists in the current working directory
    pub fn config_exists(mode: ConfigSearchMode) -> bool {
        if let Ok(folder) = Self::aptos_folder(mode) {
            let config_file = folder.join(CONFIG_FILE);
            let old_config_file = folder.join(LEGACY_CONFIG_FILE);
            config_file.exists() || old_config_file.exists()
        } else {
            false
        }
    }

    /// Loads the config from the current working directory or one of its parents.
    pub fn load(mode: ConfigSearchMode) -> CliTypedResult<Self> {
        let folder = Self::aptos_folder(mode)?;

        let config_file = folder.join(CONFIG_FILE);
        let old_config_file = folder.join(LEGACY_CONFIG_FILE);
        if config_file.exists() {
            from_yaml(
                &String::from_utf8(read_from_file(config_file.as_path())?)
                    .map_err(CliError::from)?,
            )
        } else if old_config_file.exists() {
            from_yaml(
                &String::from_utf8(read_from_file(old_config_file.as_path())?)
                    .map_err(CliError::from)?,
            )
        } else {
            Err(CliError::ConfigNotFoundError(format!(
                "{}",
                config_file.display()
            )))
        }
    }

    pub fn load_profile(
        profile: &str,
        mode: ConfigSearchMode,
    ) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load(mode)?;
        Ok(config.remove_profile(profile))
    }

    pub fn remove_profile(&mut self, profile: &str) -> Option<ProfileConfig> {
        if let Some(ref mut profiles) = self.profiles {
            profiles.remove(&profile.to_string())
        } else {
            None
        }
    }

    /// Saves the config to ./.aptos/config.yaml
    pub fn save(&self) -> CliTypedResult<()> {
        let aptos_folder = Self::aptos_folder(ConfigSearchMode::CurrentDir)?;

        // Create if it doesn't exist
        create_dir_if_not_exist(aptos_folder.as_path())?;

        // Save over previous config file
        let config_file = aptos_folder.join(CONFIG_FILE);
        let config_bytes = serde_yaml::to_string(&self).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to serialize config {}", err))
        })?;
        write_to_user_only_file(&config_file, CONFIG_FILE, config_bytes.as_bytes())?;

        // As a cleanup, delete the old if it exists
        let legacy_config_file = aptos_folder.join(LEGACY_CONFIG_FILE);
        if legacy_config_file.exists() {
            eprintln!("Removing legacy config file {}", LEGACY_CONFIG_FILE);
            let _ = std::fs::remove_file(legacy_config_file);
        }
        Ok(())
    }

    /// Finds the current directory's .aptos folder
    fn aptos_folder(mode: ConfigSearchMode) -> CliTypedResult<PathBuf> {
        let global_config = GlobalConfig::load()?;
        global_config.get_config_location(mode)
    }
}

/// Types of Keys used by the blockchain
#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum KeyType {
    /// Ed25519 key used for signing
    Ed25519,
    /// X25519 key used for network handshakes and identity
    X25519,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::X25519 => "x25519",
        };
        write!(f, "{}", str)
    }
}

impl FromStr for KeyType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ed25519" => Ok(KeyType::Ed25519),
            "x25519" => Ok(KeyType::X25519),
            _ => Err("Invalid key type: Must be one of [ed25519, x25519]"),
        }
    }
}

#[derive(Debug, Parser)]
pub struct ProfileOptions {
    /// Profile to use from the CLI config
    ///
    /// This will be used to override associated settings such as
    /// the REST URL, the Faucet URL, and the private key arguments
    #[clap(long, default_value = "default")]
    pub profile: String,
}

impl ProfileOptions {
    pub fn account_address(&self) -> CliTypedResult<AccountAddress> {
        let profile = self.profile()?;
        if let Some(account) = profile.account {
            return Ok(account);
        }

        Err(CliError::ConfigNotFoundError(self.profile.clone()))
    }

    pub fn public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        let profile = self.profile()?;
        if let Some(public_key) = profile.public_key {
            return Ok(public_key);
        }

        Err(CliError::ConfigNotFoundError(self.profile.clone()))
    }

    pub fn profile(&self) -> CliTypedResult<ProfileConfig> {
        if let Some(profile) =
            CliConfig::load_profile(&self.profile, ConfigSearchMode::CurrentDirAndParents)?
        {
            return Ok(profile);
        }

        Err(CliError::ConfigNotFoundError(self.profile.clone()))
    }
}

impl Default for ProfileOptions {
    fn default() -> Self {
        Self {
            profile: "default".to_string(),
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
        name: &'static str,
        key: &Key,
    ) -> CliTypedResult<Vec<u8>> {
        Ok(match self {
            EncodingType::Hex => hex::encode_upper(key.to_bytes()).into_bytes(),
            EncodingType::BCS => bcs::to_bytes(key).map_err(|err| CliError::BCS(name, err))?,
            EncodingType::Base64 => base64::encode(key.to_bytes()).into_bytes(),
        })
    }

    /// Loads a key from a file
    pub fn load_key<Key: ValidCryptoMaterial>(
        &self,
        name: &'static str,
        path: &Path,
    ) -> CliTypedResult<Key> {
        self.decode_key(name, read_from_file(path)?)
    }

    /// Decodes an encoded key given the known encoding
    pub fn decode_key<Key: ValidCryptoMaterial>(
        &self,
        name: &'static str,
        data: Vec<u8>,
    ) -> CliTypedResult<Key> {
        match self {
            EncodingType::BCS => bcs::from_bytes(&data).map_err(|err| CliError::BCS(name, err)),
            EncodingType::Hex => {
                let hex_string = String::from_utf8(data)?;
                Key::from_encoded_string(hex_string.trim())
                    .map_err(|err| CliError::UnableToParse(name, err.to_string()))
            }
            EncodingType::Base64 => {
                let string = String::from_utf8(data)?;
                let bytes = base64::decode(string.trim())
                    .map_err(|err| CliError::UnableToParse(name, err.to_string()))?;
                Key::try_from(bytes.as_slice()).map_err(|err| {
                    CliError::UnableToParse(name, format!("Failed to parse key {:?}", err))
                })
            }
        }
    }
}

#[derive(Clone, Debug, Parser)]
pub struct RngArgs {
    /// The seed used for key generation, should be a 64 character hex string and only used for testing
    ///
    /// If a predictable random seed is used, the key that is produced will be insecure and easy
    /// to reproduce.  Please do not use this unless sufficient randomness is put into the random
    /// seed.
    #[clap(long)]
    random_seed: Option<String>,
}

impl RngArgs {
    pub fn from_seed(seed: [u8; 32]) -> RngArgs {
        RngArgs {
            random_seed: Some(hex::encode(seed)),
        }
    }

    /// Returns a key generator with the seed if given
    pub fn key_generator(&self) -> CliTypedResult<KeyGen> {
        if let Some(ref seed) = self.random_seed {
            // Strip 0x
            let seed = seed.strip_prefix("0x").unwrap_or(seed);
            let mut seed_slice = [0u8; 32];

            hex::decode_to_slice(seed, &mut seed_slice)?;
            Ok(KeyGen::from_seed(seed_slice))
        } else {
            Ok(KeyGen::from_os_rng())
        }
    }
}

impl Default for EncodingType {
    fn default() -> Self {
        EncodingType::Hex
    }
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

/// An insertable option for use with prompts.
#[derive(Clone, Copy, Debug, Default, Parser)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_yes: bool,
    /// Assume no for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_no: bool,
}

impl PromptOptions {
    pub fn yes() -> Self {
        Self {
            assume_yes: true,
            assume_no: false,
        }
    }

    pub fn no() -> Self {
        Self {
            assume_yes: false,
            assume_no: true,
        }
    }
}

/// An insertable option for use with encodings.
#[derive(Debug, Default, Parser)]
pub struct EncodingOptions {
    /// Encoding of data as one of [base64, bcs, hex]
    #[clap(long, default_value_t = EncodingType::Hex)]
    pub encoding: EncodingType,
}

#[derive(Debug, Parser)]
pub struct PublicKeyInputOptions {
    /// Public key input file name
    #[clap(long, group = "public_key_input", parse(from_os_str))]
    public_key_file: Option<PathBuf>,
    /// Public key encoded in a type as shown in `encoding`
    #[clap(long, group = "public_key_input")]
    public_key: Option<String>,
}

impl PublicKeyInputOptions {
    pub fn from_key(key: &Ed25519PublicKey) -> PublicKeyInputOptions {
        PublicKeyInputOptions {
            public_key: Some(key.to_encoded_string().unwrap()),
            public_key_file: None,
        }
    }
}

impl ExtractPublicKey for PublicKeyInputOptions {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<Ed25519PublicKey> {
        if let Some(ref file) = self.public_key_file {
            encoding.load_key("--public-key-file", file.as_path())
        } else if let Some(ref key) = self.public_key {
            let key = key.as_bytes().to_vec();
            encoding.decode_key("--public-key", key)
        } else if let Some(Some(public_key)) =
            CliConfig::load_profile(profile, ConfigSearchMode::CurrentDirAndParents)?
                .map(|p| p.public_key)
        {
            Ok(public_key)
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--public-key', '--public-key-file', '--profile'] must be used"
                    .to_string(),
            ))
        }
    }
}

pub trait ParsePrivateKey {
    fn parse_private_key(
        &self,
        encoding: EncodingType,
        private_key_file: Option<PathBuf>,
        private_key: Option<String>,
    ) -> CliTypedResult<Option<Ed25519PrivateKey>> {
        if let Some(ref file) = private_key_file {
            Ok(Some(
                encoding.load_key("--private-key-file", file.as_path())?,
            ))
        } else if let Some(ref key) = private_key {
            let key = key.as_bytes().to_vec();
            Ok(Some(encoding.decode_key("--private-key", key)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Default, Parser)]
pub struct PrivateKeyInputOptions {
    /// Private key input file name
    #[clap(long, group = "private_key_input", parse(from_os_str))]
    private_key_file: Option<PathBuf>,
    /// Private key encoded in a type as shown in `encoding`
    #[clap(long, group = "private_key_input")]
    private_key: Option<String>,
}

impl ParsePrivateKey for PrivateKeyInputOptions {}

impl PrivateKeyInputOptions {
    pub fn from_private_key(private_key: &Ed25519PrivateKey) -> CliTypedResult<Self> {
        Ok(PrivateKeyInputOptions {
            private_key: Some(
                private_key
                    .to_encoded_string()
                    .map_err(|err| CliError::UnexpectedError(err.to_string()))?,
            ),
            private_key_file: None,
        })
    }

    pub fn from_x25519_private_key(private_key: &x25519::PrivateKey) -> CliTypedResult<Self> {
        Ok(PrivateKeyInputOptions {
            private_key: Some(
                private_key
                    .to_encoded_string()
                    .map_err(|err| CliError::UnexpectedError(err.to_string()))?,
            ),
            private_key_file: None,
        })
    }

    pub fn from_file(file: PathBuf) -> Self {
        PrivateKeyInputOptions {
            private_key: None,
            private_key_file: Some(file),
        }
    }

    /// Extract private key from CLI args with fallback to config
    pub fn extract_private_key_and_address(
        &self,
        encoding: EncodingType,
        profile: &str,
        maybe_address: Option<AccountAddress>,
    ) -> CliTypedResult<(Ed25519PrivateKey, AccountAddress)> {
        // Order of operations
        // 1. CLI inputs
        // 2. Profile
        // 3. Derived
        if let Some(key) = self.extract_private_key_cli(encoding)? {
            // If we use the CLI inputs, then we should derive or use the address from the input
            if let Some(address) = maybe_address {
                Ok((key, address))
            } else {
                let address = account_address_from_public_key(&key.public_key());
                Ok((key, address))
            }
        } else if let Some((Some(key), maybe_config_address)) =
            CliConfig::load_profile(profile, ConfigSearchMode::CurrentDirAndParents)?
                .map(|p| (p.private_key, p.account))
        {
            match (maybe_address, maybe_config_address) {
                (Some(address), _) => Ok((key, address)),
                (_, Some(address)) => Ok((key, address)),
                (None, None) => {
                    let address = account_address_from_public_key(&key.public_key());
                    Ok((key, address))
                }
            }
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'] must be used".to_string(),
            ))
        }
    }

    /// Extract private key from CLI args with fallback to config
    pub fn extract_private_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<Ed25519PrivateKey> {
        if let Some(key) = self.extract_private_key_cli(encoding)? {
            Ok(key)
        } else if let Some(Some(private_key)) =
            CliConfig::load_profile(profile, ConfigSearchMode::CurrentDirAndParents)?
                .map(|p| p.private_key)
        {
            Ok(private_key)
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'] must be used".to_string(),
            ))
        }
    }

    /// Extract private key from CLI args
    pub fn extract_private_key_cli(
        &self,
        encoding: EncodingType,
    ) -> CliTypedResult<Option<Ed25519PrivateKey>> {
        self.parse_private_key(
            encoding,
            self.private_key_file.clone(),
            self.private_key.clone(),
        )
    }
}

impl ExtractPublicKey for PrivateKeyInputOptions {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<Ed25519PublicKey> {
        self.extract_private_key(encoding, profile)
            .map(|private_key| private_key.public_key())
    }
}

pub trait ExtractPublicKey {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<Ed25519PublicKey>;
}

pub fn account_address_from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    let auth_key = AuthenticationKey::ed25519(public_key);
    AccountAddress::new(*auth_key.derived_address())
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
    pub fn check_file(&self) -> CliTypedResult<()> {
        check_if_file_exists(self.output_file.as_path(), self.prompt_options)
    }

    /// Save to the `output_file`
    pub fn save_to_file(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        write_to_file(self.output_file.as_path(), name, bytes)
    }

    /// Save to the `output_file` with restricted permissions (mode 0600)
    pub fn save_to_file_confidential(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        let mut opts = OpenOptions::new();
        #[cfg(unix)]
        opts.mode(0o600);
        write_to_file_with_opts(self.output_file.as_path(), name, bytes, &mut opts)
    }
}

/// Options specific to using the Rest endpoint
#[derive(Debug, Default, Parser)]
pub struct RestOptions {
    /// URL to a fullnode on the network
    ///
    /// Defaults to <https://fullnode.devnet.aptoslabs.com/v1>
    #[clap(long)]
    pub(crate) url: Option<reqwest::Url>,
}

impl RestOptions {
    pub fn new(url: Option<reqwest::Url>) -> Self {
        RestOptions { url }
    }

    /// Retrieve the URL from the profile or the command line
    pub fn url(&self, profile: &str) -> CliTypedResult<reqwest::Url> {
        if let Some(ref url) = self.url {
            Ok(url.clone())
        } else if let Some(Some(url)) =
            CliConfig::load_profile(profile, ConfigSearchMode::CurrentDirAndParents)?
                .map(|p| p.rest_url)
        {
            reqwest::Url::parse(&url)
                .map_err(|err| CliError::UnableToParse("Rest URL", err.to_string()))
        } else {
            reqwest::Url::parse(DEFAULT_REST_URL).map_err(|err| {
                CliError::UnexpectedError(format!("Failed to parse default rest URL {}", err))
            })
        }
    }

    pub fn client(&self, profile: &str) -> CliTypedResult<Client> {
        Ok(Client::new(self.url(profile)?))
    }
}

/// Options for compiling a move package dir
#[derive(Debug, Parser)]
pub struct MovePackageDir {
    /// Path to a move package (the folder with a Move.toml file)
    #[clap(long, parse(from_os_str))]
    pub package_dir: Option<PathBuf>,
    /// Path to save the compiled move package
    ///
    /// Defaults to `<package_dir>/build`
    #[clap(long, parse(from_os_str))]
    pub output_dir: Option<PathBuf>,
    /// Named addresses for the move binary
    ///
    /// Example: alice=0x1234, bob=0x5678
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, parse(try_from_str = crate::common::utils::parse_map), default_value = "")]
    pub(crate) named_addresses: BTreeMap<String, AccountAddressWrapper>,
}

impl MovePackageDir {
    pub fn new(package_dir: PathBuf) -> Self {
        Self {
            package_dir: Some(package_dir),
            output_dir: None,
            named_addresses: Default::default(),
        }
    }

    pub fn get_package_path(&self) -> CliTypedResult<PathBuf> {
        dir_default_to_current(self.package_dir.clone())
    }

    /// Retrieve the NamedAddresses, resolving all the account addresses accordingly
    pub fn named_addresses(&self) -> BTreeMap<String, AccountAddress> {
        self.named_addresses
            .clone()
            .into_iter()
            .map(|(key, value)| (key, value.account_address))
            .collect()
    }
}

/// A wrapper around `AccountAddress` to be more flexible from strings than AccountAddress
#[derive(Clone, Copy, Debug)]
pub struct AccountAddressWrapper {
    pub account_address: AccountAddress,
}

impl FromStr for AccountAddressWrapper {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AccountAddressWrapper {
            account_address: load_account_arg(s)?,
        })
    }
}

/// Loads an account arg and allows for naming based on profiles
pub fn load_account_arg(str: &str) -> Result<AccountAddress, CliError> {
    if str.starts_with("0x") {
        AccountAddress::from_hex_literal(str).map_err(|err| {
            CliError::CommandArgumentError(format!("Failed to parse AccountAddress {}", err))
        })
    } else if let Ok(account_address) = AccountAddress::from_str(str) {
        Ok(account_address)
    } else if let Some(Some(private_key)) =
        CliConfig::load_profile(str, ConfigSearchMode::CurrentDirAndParents)?.map(|p| p.private_key)
    {
        let public_key = private_key.public_key();
        Ok(account_address_from_public_key(&public_key))
    } else {
        Err(CliError::CommandArgumentError(
            "'--account' or '--profile' after using aptos init must be provided".to_string(),
        ))
    }
}

/// A wrapper around `AccountAddress` to allow for "_"
#[derive(Clone, Copy, Debug)]
pub struct MoveManifestAccountWrapper {
    pub account_address: Option<AccountAddress>,
}

impl FromStr for MoveManifestAccountWrapper {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MoveManifestAccountWrapper {
            account_address: load_manifest_account_arg(s)?,
        })
    }
}

/// Loads an account arg and allows for naming based on profiles and "_"
pub fn load_manifest_account_arg(str: &str) -> Result<Option<AccountAddress>, CliError> {
    if str == "_" {
        Ok(None)
    } else if str.starts_with("0x") {
        AccountAddress::from_hex_literal(str)
            .map(Some)
            .map_err(|err| {
                CliError::CommandArgumentError(format!("Failed to parse AccountAddress {}", err))
            })
    } else if let Ok(account_address) = AccountAddress::from_str(str) {
        Ok(Some(account_address))
    } else if let Some(Some(private_key)) =
        CliConfig::load_profile(str, ConfigSearchMode::CurrentDirAndParents)?.map(|p| p.private_key)
    {
        let public_key = private_key.public_key();
        Ok(Some(account_address_from_public_key(&public_key)))
    } else {
        Err(CliError::CommandArgumentError(
            "Invalid Move manifest account address".to_string(),
        ))
    }
}

/// A common trait for all CLI commands to have consistent outputs
#[async_trait]
pub trait CliCommand<T: Serialize + Send>: Sized + Send {
    /// Returns a name for logging purposes
    fn command_name(&self) -> &'static str;

    /// Executes the command, returning a command specific type
    async fn execute(self) -> CliTypedResult<T>;

    /// Executes the command, and serializes it to the common JSON output type
    async fn execute_serialized(self) -> CliResult {
        let command_name = self.command_name();
        start_logger();
        let start_time = Instant::now();
        to_common_result(command_name, start_time, self.execute().await).await
    }

    /// Same as execute serialized without setting up logging
    async fn execute_serialized_without_logger(self) -> CliResult {
        let command_name = self.command_name();
        let start_time = Instant::now();
        to_common_result(command_name, start_time, self.execute().await).await
    }

    /// Executes the command, and throws away Ok(result) for the string Success
    async fn execute_serialized_success(self) -> CliResult {
        start_logger();
        let command_name = self.command_name();
        let start_time = Instant::now();
        to_common_success_result(command_name, start_time, self.execute().await).await
    }
}

/// A shortened transaction output
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TransactionSummary {
    pub transaction_hash: HashValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_unit_price: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_number: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_us: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_status: Option<String>,
}

impl From<Transaction> for TransactionSummary {
    fn from(transaction: Transaction) -> Self {
        TransactionSummary::from(&transaction)
    }
}
impl From<&Transaction> for TransactionSummary {
    fn from(transaction: &Transaction) -> Self {
        match transaction {
            Transaction::PendingTransaction(txn) => TransactionSummary {
                transaction_hash: txn.hash,
                pending: Some(true),
                sender: Some(*txn.request.sender.inner()),
                sequence_number: Some(txn.request.sequence_number.0),
                gas_used: None,
                gas_unit_price: None,
                success: None,
                version: None,
                vm_status: None,
                timestamp_us: None,
            },
            Transaction::UserTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                sender: Some(*txn.request.sender.inner()),
                gas_used: Some(txn.info.gas_used.0),
                gas_unit_price: Some(txn.request.gas_unit_price.0),
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                sequence_number: Some(txn.request.sequence_number.0),
                timestamp_us: Some(txn.timestamp.0),
                pending: None,
            },
            Transaction::GenesisTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
                timestamp_us: None,
            },
            Transaction::BlockMetadataTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
            },
            Transaction::StateCheckpointTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sequence_number: None,
            },
        }
    }
}

/// A summary of a [`WriteSetChange`] for easy printing
#[derive(Clone, Debug, Default, Serialize)]
pub struct ChangeSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    event: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Debug, Default, Parser)]
pub struct FaucetOptions {
    /// URL for the faucet endpoint e.g. https://faucet.devnet.aptoslabs.com
    #[clap(long)]
    faucet_url: Option<reqwest::Url>,
}

impl FaucetOptions {
    pub fn new(faucet_url: Option<reqwest::Url>) -> Self {
        FaucetOptions { faucet_url }
    }

    pub fn faucet_url(&self, profile: &str) -> CliTypedResult<reqwest::Url> {
        if let Some(ref faucet_url) = self.faucet_url {
            Ok(faucet_url.clone())
        } else if let Some(Some(url)) =
            CliConfig::load_profile(profile, ConfigSearchMode::CurrentDirAndParents)?
                .map(|profile| profile.faucet_url)
        {
            reqwest::Url::parse(&url)
                .map_err(|err| CliError::UnableToParse("config faucet_url", err.to_string()))
        } else {
            reqwest::Url::parse(DEFAULT_FAUCET_URL).map_err(|err| {
                CliError::UnexpectedError(format!("Failed to parse default faucet URL {}", err))
            })
        }
    }
}

/// Gas price options for manipulating how to prioritize transactions
#[derive(Debug, Default, Eq, Parser, PartialEq)]
pub struct GasOptions {
    /// Gas multiplier per unit of gas
    ///
    /// The amount of Octas (10^-8 APT) used for a transaction is equal
    /// to (gas unit price * gas used).  The gas_unit_price can
    /// be used as a multiplier for the amount of Octas willing
    /// to be paid for a transaction.  This will prioritize the
    /// transaction with a higher gas unit price.
    ///
    /// Without a value, it will determine the price based on the current estimated price
    #[clap(long)]
    pub gas_unit_price: Option<u64>,
    /// Maximum amount of gas units to be used to send this transaction
    ///
    /// The maximum amount of gas units willing to pay for the transaction.
    /// This is the (max gas in Octas / gas unit price).
    ///
    /// For example if I wanted to pay a maximum of 100 Octas, I may have the
    /// max gas set to 100 if the gas unit price is 1.  If I want it to have a
    /// gas unit price of 2, the max gas would need to be 50 to still only have
    /// a maximum price of 100 Octas.
    ///
    /// Without a value, it will determine the price based on simulating the current transaction
    #[clap(long)]
    pub max_gas: Option<u64>,
}

/// Common options for interacting with an account for a validator
#[derive(Debug, Default, Parser)]
pub struct TransactionOptions {
    /// [Deprecated] Estimate maximum gas via simulation
    ///
    /// Deprecated parameter, the default behavior is now to estimate max gas automatically, and ask for
    /// confirmation
    ///
    /// This will simulate the transaction, and use the simulated actual amount of gas
    /// to be used as the max gas.
    #[clap(long)]
    pub(crate) estimate_max_gas: bool,

    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) sender_account: Option<AccountAddress>,

    #[clap(flatten)]
    pub(crate) private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) gas_options: GasOptions,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

impl TransactionOptions {
    /// Builds a rest client
    fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options.profile)
    }

    /// Retrieves the public key and the associated address
    /// TODO: Cache this information
    pub fn get_key_and_address(&self) -> CliTypedResult<(Ed25519PrivateKey, AccountAddress)> {
        self.private_key_options.extract_private_key_and_address(
            self.encoding_options.encoding,
            &self.profile_options.profile,
            self.sender_account,
        )
    }

    pub fn sender_address(&self) -> CliTypedResult<AccountAddress> {
        Ok(self.get_key_and_address()?.1)
    }

    /// Gets the auth key by account address. We need to fetch the auth key from Rest API rather than creating an
    /// auth key out of the public key.
    pub(crate) async fn auth_key(
        &self,
        sender_address: AccountAddress,
    ) -> CliTypedResult<AuthenticationKey> {
        let client = self.rest_client()?;
        get_auth_key(&client, sender_address).await
    }

    pub async fn sequence_number(&self, sender_address: AccountAddress) -> CliTypedResult<u64> {
        let client = self.rest_client()?;
        get_sequence_number(&client, sender_address).await
    }

    /// Submit a transaction
    pub async fn submit_transaction(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<Transaction> {
        let client = self.rest_client()?;
        let (sender_key, sender_address) = self.get_key_and_address()?;

        // Get sequence number for account
        let sequence_number = self.sequence_number(sender_address).await?;

        // Ask to confirm price if the gas unit price is estimated above the lowest value when
        // it is automatically estimated
        let ask_to_confirm_price;
        let gas_unit_price = if let Some(gas_unit_price) = self.gas_options.gas_unit_price {
            ask_to_confirm_price = false;
            gas_unit_price
        } else {
            let gas_unit_price = client.estimate_gas_price().await?.into_inner().gas_estimate;

            ask_to_confirm_price = true;
            gas_unit_price
        };

        let max_gas = if let Some(max_gas) = self.gas_options.max_gas {
            // If the gas unit price was estimated ask, but otherwise you've chosen hwo much you want to spend
            if ask_to_confirm_price {
                let message = format!("Do you want to submit transaction for a maximum of {} Octas at a gas unit price of {} Octas?",  max_gas * gas_unit_price, gas_unit_price);
                prompt_yes_with_override(&message, self.prompt_options)?;
            }
            max_gas
        } else {
            let transaction_factory = TransactionFactory::new(chain_id(&client).await?)
                .with_gas_unit_price(gas_unit_price);

            let unsigned_transaction = transaction_factory
                .payload(payload.clone())
                .sender(sender_address)
                .sequence_number(sequence_number)
                .build();

            let signed_transaction = SignedTransaction::new(
                unsigned_transaction,
                sender_key.public_key(),
                Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
            );
            // TODO: Cleanup to use the gas price estimation here
            let simulated_txn = client
                .simulate_bcs_with_gas_estimation(&signed_transaction, true, false)
                .await?
                .into_inner();

            // Check if the transaction will pass, if it doesn't then fail
            // TODO: Add move resolver so we can explain the VM status with a proper error map
            let status = simulated_txn.info.status();
            if !status.is_success() {
                let status = client.explain_vm_status(status);
                return Err(CliError::SimulationError(status));
            }

            // Take the gas used and use a headroom factor on it
            let adjusted_max_gas = adjust_gas_headroom(
                simulated_txn.info.gas_used(),
                simulated_txn
                    .transaction
                    .as_signed_user_txn()
                    .expect("Should be signed user transaction")
                    .max_gas_amount(),
            );

            // Ask if you want to accept the estimate amount
            let upper_cost_bound = adjusted_max_gas * gas_unit_price;
            let lower_cost_bound = simulated_txn.info.gas_used() * gas_unit_price;
            let message = format!(
                    "Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
                    lower_cost_bound,
                    upper_cost_bound,
                    gas_unit_price);
            prompt_yes_with_override(&message, self.prompt_options)?;
            adjusted_max_gas
        };

        // Sign and submit transaction
        let transaction_factory = TransactionFactory::new(chain_id(&client).await?)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
        let response = client
            .submit_and_wait(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;

        Ok(response.into_inner())
    }

    pub async fn simulate_transaction(
        &self,
        payload: TransactionPayload,
        gas_price: Option<u64>,
        amount_transfer: Option<u64>,
    ) -> CliTypedResult<UserTransaction> {
        let client = self.rest_client()?;
        let (sender_key, sender_address) = self.get_key_and_address()?;

        // Get sequence number for account
        let sequence_number = get_sequence_number(&client, sender_address).await?;

        // Estimate gas price if necessary
        let gas_price = if let Some(gas_price) = gas_price {
            gas_price
        } else {
            self.estimate_gas_price().await?
        };
        // Simulate transaction
        // To get my known possible max gas, I need to get my current balance
        let account_balance = client
            .get_account_balance(sender_address)
            .await?
            .into_inner()
            .coin
            .value
            .0;

        let max_possible_gas = if gas_price == 0 {
            MAX_POSSIBLE_GAS_UNITS
        } else if let Some(amount) = amount_transfer {
            std::cmp::min(
                account_balance
                    .saturating_sub(amount)
                    .saturating_div(gas_price),
                MAX_POSSIBLE_GAS_UNITS,
            )
        } else {
            std::cmp::min(
                account_balance.saturating_div(gas_price),
                MAX_POSSIBLE_GAS_UNITS,
            )
        };

        let transaction_factory = TransactionFactory::new(chain_id(&client).await?)
            .with_gas_unit_price(gas_price)
            .with_max_gas_amount(max_possible_gas);

        let unsigned_transaction = transaction_factory
            .payload(payload)
            .sender(sender_address)
            .sequence_number(sequence_number)
            .build();

        let signed_transaction = SignedTransaction::new(
            unsigned_transaction,
            sender_key.public_key(),
            Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
        );
        let txns = client.simulate(&signed_transaction).await?.into_inner();
        Ok(txns.first().unwrap().clone())
    }

    pub async fn estimate_gas_price(&self) -> CliTypedResult<u64> {
        let client = self.rest_client()?;
        client
            .estimate_gas_price()
            .await
            .map(|inner| inner.into_inner().gas_estimate)
            .map_err(|err| {
                CliError::UnexpectedError(format!(
                    "Failed to retrieve gas price estimate {:?}",
                    err
                ))
            })
    }
}

#[derive(Parser)]
pub struct OptionalPoolAddressArgs {
    /// Address of the Staking pool
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) pool_address: Option<AccountAddress>,
}

#[derive(Parser)]
pub struct PoolAddressArgs {
    /// Address of the Staking pool
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) pool_address: AccountAddress,
}

// This struct includes TypeInfo (account_address, module_name, and struct_name)
// and RotationProofChallenge-specific information (sequence_number, originator, current_auth_key, and new_public_key)
// Since the struct RotationProofChallenge is defined in "0x1::account::RotationProofChallenge",
// we will be passing in "0x1" to `account_address`, "account" to `module_name`, and "RotationProofChallenge" to `struct_name`
// Originator refers to the user's address
#[derive(Serialize, Deserialize)]
pub struct RotationProofChallenge {
    // Should be `CORE_CODE_ADDRESS`
    pub account_address: AccountAddress,
    // Should be `account`
    pub module_name: String,
    // Should be `RotationProofChallenge`
    pub struct_name: String,
    pub sequence_number: u64,
    pub originator: AccountAddress,
    pub current_auth_key: AccountAddress,
    pub new_public_key: Vec<u8>,
}
