// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        init::DEFAULT_REST_URL,
        utils::{
            check_if_file_exists, read_from_file, to_common_result, to_common_success_result,
            write_to_file,
        },
    },
    genesis::git::from_yaml,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    x25519, PrivateKey, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
};
use aptos_logger::debug;
use aptos_rest_client::{aptos_api_types::WriteSetChange, Client, Transaction};
use aptos_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use async_trait::async_trait;
use clap::{ArgEnum, Parser};
use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    path::{Path, PathBuf},
    str::FromStr,
    time::Instant,
};
use thiserror::Error;

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
    #[error("Move unit tests failed: {0}")]
    MoveTestError(String),
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
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
            CliError::MoveTestError(_) => "MoveTestError",
            CliError::UnableToParse(_, _) => "UnableToParse",
            CliError::UnableToReadFile(_, _) => "UnableToReadFile",
            CliError::UnexpectedError(_) => "UnexpectedError",
        }
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

/// Config saved to `.aptos/config.yaml`
#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    /// Map of profile configs
    pub profiles: Option<HashMap<String, ProfileConfig>>,
}

const CONFIG_FILE: &str = "config.yaml";
const LEGACY_CONFIG_FILE: &str = "config.yml";
const CONFIG_FOLDER: &str = ".aptos";

/// An individual profile
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Private key for commands.
    pub private_key: Option<Ed25519PrivateKey>,
    /// Public key for commands
    pub public_key: Option<Ed25519PublicKey>,
    /// Account for commands
    pub account: Option<AccountAddress>,
    /// URL for the Aptos rest endpoint
    pub rest_url: Option<String>,
    /// URL for the Faucet endpoint (if applicable)
    pub faucet_url: Option<String>,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            profiles: Some(HashMap::new()),
        }
    }
}

impl CliConfig {
    /// Checks if the config exists in the current working directory
    pub fn config_exists() -> bool {
        if let Ok(folder) = Self::aptos_folder() {
            let config_file = folder.join(CONFIG_FILE);
            let old_config_file = folder.join(LEGACY_CONFIG_FILE);
            config_file.exists() || old_config_file.exists()
        } else {
            false
        }
    }

    /// Loads the config from the current working directory
    pub fn load() -> CliTypedResult<Self> {
        let folder = Self::aptos_folder()?;

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

    pub fn load_profile(profile: &str) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load()?;
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
        let aptos_folder = Self::aptos_folder()?;

        // Create if it doesn't exist
        if !aptos_folder.exists() {
            std::fs::create_dir(&aptos_folder).map_err(|err| {
                CliError::CommandArgumentError(format!(
                    "Unable to create {} directory {}",
                    aptos_folder.display(),
                    err
                ))
            })?;
            debug!("Created {} folder", aptos_folder.display());
        } else {
            debug!("{} folder already initialized", aptos_folder.display());
        }

        // Save over previous config file
        let config_file = aptos_folder.join(CONFIG_FILE);
        let config_bytes = serde_yaml::to_string(&self).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to serialize config {}", err))
        })?;
        write_to_file(&config_file, CONFIG_FILE, config_bytes.as_bytes())?;

        // As a cleanup, delete the old if it exists
        let legacy_config_file = aptos_folder.join(LEGACY_CONFIG_FILE);
        if legacy_config_file.exists() {
            eprintln!("Removing legacy config file {}", LEGACY_CONFIG_FILE);
            let _ = std::fs::remove_file(legacy_config_file);
        }
        Ok(())
    }

    /// Finds the current directory's .aptos folder
    fn aptos_folder() -> CliTypedResult<PathBuf> {
        std::env::current_dir()
            .map_err(|err| {
                CliError::UnexpectedError(format!("Unable to get current directory {}", err))
            })
            .map(|dir| dir.join(CONFIG_FOLDER))
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

#[derive(Debug, Parser)]
pub struct ProfileOptions {
    /// Profile to use from config
    #[clap(long, default_value = "default")]
    pub profile: String,
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
                let hex_string = String::from_utf8(data).unwrap();
                Key::from_encoded_string(hex_string.trim())
                    .map_err(|err| CliError::UnableToParse(name, err.to_string()))
            }
            EncodingType::Base64 => {
                let string = String::from_utf8(data).unwrap();
                let bytes = base64::decode(string.trim())
                    .map_err(|err| CliError::UnableToParse(name, err.to_string()))?;
                Key::try_from(bytes.as_slice()).map_err(|err| {
                    CliError::UnableToParse(name, format!("Failed to parse key {:?}", err))
                })
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
#[derive(Clone, Copy, Debug, Parser)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_yes: bool,
    /// Assume no for all yes/no prompts
    #[clap(long, group = "prompt_options")]
    pub assume_no: bool,
}

/// An insertable option for use with encodings.
#[derive(Debug, Parser)]
pub struct EncodingOptions {
    /// Encoding of data as `base64`, `bcs`, or `hex`
    #[clap(long, default_value = "hex")]
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

impl ExtractPublicKey for PublicKeyInputOptions {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        _profile: &str,
    ) -> CliTypedResult<Ed25519PublicKey> {
        if let Some(ref file) = self.public_key_file {
            encoding.load_key("--public-key-file", file.as_path())
        } else if let Some(ref key) = self.public_key {
            let key = key.as_bytes().to_vec();
            encoding.decode_key("--public-key", key)
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--public-key', '--public-key-file'] must be used".to_string(),
            ))
        }
    }
}

#[derive(Debug, Parser)]
pub struct PrivateKeyInputOptions {
    /// Private key input file name
    #[clap(long, group = "private_key_input", parse(from_os_str))]
    private_key_file: Option<PathBuf>,
    /// Private key encoded in a type as shown in `encoding`
    #[clap(long, group = "private_key_input")]
    private_key: Option<String>,
}

impl PrivateKeyInputOptions {
    /// Extract private key from CLI args with fallback to config
    pub fn extract_private_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<Ed25519PrivateKey> {
        if let Some(key) = self.extract_private_key_cli(encoding)? {
            Ok(key)
        } else if let Some(Some(private_key)) =
            CliConfig::load_profile(profile)?.map(|p| p.private_key)
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
        if let Some(ref file) = self.private_key_file {
            Ok(Some(
                encoding.load_key("--private-key-file", file.as_path())?,
            ))
        } else if let Some(ref key) = self.private_key {
            let key = key.as_bytes().to_vec();
            Ok(Some(encoding.decode_key("--private-key", key)?))
        } else {
            Ok(None)
        }
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

    fn extract_x25519_public_key(
        &self,
        encoding: EncodingType,
        profile: &str,
    ) -> CliTypedResult<x25519::PublicKey> {
        let key = self.extract_public_key(encoding, profile)?;
        x25519::PublicKey::from_ed25519_public_bytes(&key.to_bytes()).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to convert ed25519 to x25519 {:?}", err))
        })
    }
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
}

/// Options specific to using the Rest endpoint
#[derive(Debug, Parser)]
pub struct RestOptions {
    /// URL to a fullnode on the network
    ///
    /// Defaults to https://fullnode.devnet.aptoslabs.com
    #[clap(long, parse(try_from_str))]
    pub url: Option<reqwest::Url>,
}

impl RestOptions {
    /// Retrieve the URL from the profile or the command line
    pub fn url(&self, profile: &str) -> CliTypedResult<reqwest::Url> {
        if let Some(ref url) = self.url {
            Ok(url.clone())
        } else if let Some(Some(url)) = CliConfig::load_profile(profile)?.map(|p| p.rest_url) {
            reqwest::Url::parse(&url)
                .map_err(|err| CliError::UnableToParse("Rest URL", err.to_string()))
        } else {
            reqwest::Url::parse(DEFAULT_REST_URL).map_err(|err| {
                CliError::UnexpectedError(format!("Failed to parse default rest URL {}", err))
            })
        }
    }
}

/// Options specific to submitting a private key to the Rest endpoint
#[derive(Debug, Parser)]
pub struct WriteTransactionOptions {
    #[clap(flatten)]
    pub private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub rest_options: RestOptions,
    /// Maximum gas to be used to publish the package
    ///
    /// Defaults to 1000 gas units
    #[clap(long, default_value_t = 1000)]
    pub max_gas: u64,
}

impl WriteTransactionOptions {
    /// Retrieve the chain id from onchain via the Rest API
    pub async fn chain_id(&self, profile: &str) -> CliTypedResult<ChainId> {
        let client = Client::new(self.rest_options.url(profile)?);
        let state = client
            .get_ledger_information()
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner();
        Ok(ChainId::new(state.chain_id))
    }
}

/// Options for compiling a move package dir
#[derive(Debug, Parser)]
pub struct MovePackageDir {
    /// Path to a move package (the folder with a Move.toml file)
    #[clap(long, parse(from_os_str), default_value = ".")]
    pub package_dir: PathBuf,
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
    named_addresses: BTreeMap<String, AccountAddressWrapper>,
}

impl MovePackageDir {
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
    } else if let Some(Some(private_key)) = CliConfig::load_profile(str)?.map(|p| p.private_key) {
        let public_key = private_key.public_key();
        Ok(account_address_from_public_key(&public_key))
    } else {
        Err(CliError::CommandArgumentError(
            "'--account-address' or '--profile' after using aptos init must be provided"
                .to_string(),
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
        let start_time = Instant::now();
        to_common_result(command_name, start_time, self.execute().await).await
    }

    /// Executes the command, and throws away Ok(result) for the string Success
    async fn execute_serialized_success(self) -> CliResult {
        let command_name = self.command_name();
        let start_time = Instant::now();
        to_common_success_result(command_name, start_time, self.execute().await).await
    }
}

/// A shortened transaction output
#[derive(Clone, Debug, Default, Serialize)]
pub struct TransactionSummary {
    changes: Vec<ChangeSummary>,
    gas_used: Option<u64>,
    success: bool,
    version: Option<u64>,
    vm_status: String,
}

impl From<Transaction> for TransactionSummary {
    fn from(transaction: Transaction) -> Self {
        let mut summary = TransactionSummary {
            success: transaction.success(),
            version: transaction.version(),
            vm_status: transaction.vm_status(),
            ..Default::default()
        };

        if let Ok(info) = transaction.transaction_info() {
            summary.gas_used = Some(info.gas_used.0);
            summary.changes = info
                .changes
                .iter()
                .map(|change| match change {
                    WriteSetChange::DeleteModule { module, .. } => ChangeSummary {
                        event: change.type_str(),
                        module: Some(module.to_string()),
                        ..Default::default()
                    },
                    WriteSetChange::DeleteResource {
                        address, resource, ..
                    } => ChangeSummary {
                        event: change.type_str(),
                        address: Some(*address.inner()),
                        resource: Some(resource.to_string()),
                        ..Default::default()
                    },
                    WriteSetChange::DeleteTableItem { handle, key, .. } => ChangeSummary {
                        event: change.type_str(),
                        handle: Some(handle.to_string()),
                        key: Some(key.to_string()),
                        ..Default::default()
                    },
                    WriteSetChange::WriteModule { address, .. } => ChangeSummary {
                        event: change.type_str(),
                        address: Some(*address.inner()),
                        ..Default::default()
                    },
                    WriteSetChange::WriteResource { address, data, .. } => ChangeSummary {
                        event: change.type_str(),
                        address: Some(*address.inner()),
                        resource: Some(data.typ.to_string()),
                        data: Some(serde_json::to_value(&data.data).unwrap_or_default()),
                        ..Default::default()
                    },
                    WriteSetChange::WriteTableItem {
                        handle, key, value, ..
                    } => ChangeSummary {
                        event: change.type_str(),
                        handle: Some(handle.to_string()),
                        key: Some(key.to_string()),
                        value: Some(value.to_string()),
                        ..Default::default()
                    },
                })
                .collect();
        }

        summary
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
