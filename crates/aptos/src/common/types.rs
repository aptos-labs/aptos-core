// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::utils::{explorer_transaction_link, fund_account, strip_private_key_prefix};
use crate::{
    common::{
        init::Network,
        local_simulation,
        transactions::ReplayProtectionType,
        utils::{
            check_if_file_exists, create_dir_if_not_exist, deserialize_address_str,
            deserialize_material_with_prefix, dir_default_to_current, get_account_with_state,
            get_auth_key, get_sequence_number, parse_json_file, prompt_yes_with_override,
            read_from_file, serialize_material_with_prefix, start_logger, to_common_result,
            to_common_success_result, write_to_file, write_to_file_with_opts,
            write_to_user_only_file,
        },
    },
    config::GlobalConfig,
    genesis::git::from_yaml,
    move_tool::{ArgWithType, FunctionArgType, MemberId},
};
use anyhow::{bail, Context};
use aptos_api_types::ViewFunction;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature},
    encoding_type::{EncodingError, EncodingType},
    x25519, PrivateKey, ValidCryptoMaterialStringExt,
};
use aptos_framework::chunked_publish::{
    default_large_packages_module_address, CHUNK_SIZE_IN_BYTES,
};
use aptos_global_constants::adjust_gas_headroom;
use aptos_keygen::KeyGen;
use aptos_logger::Level;
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{
    aptos_api_types::{EntryFunctionId, HashValue, MoveType, ViewRequest},
    error::RestError,
    AptosBaseUrl, Client, Transaction,
};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{HardwareWalletAccount, HardwareWalletType, LocalAccount, TransactionSigner},
};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, EntryFunction, MultisigTransactionPayload,
        ReplayProtector, Script, SignedTransaction, TransactionArgument, TransactionPayload,
        TransactionStatus,
    },
};
use aptos_vm_types::output::VMOutput;
use async_trait::async_trait;
use clap::{Parser, ValueEnum};
use hex::FromHexError;
use indoc::indoc;
use move_compiler_v2::Experiment;
use move_core_types::{
    account_address::AccountAddress, language_storage::TypeTag, vm_status::VMStatus,
};
use move_model::metadata::{
    CompilerVersion, LanguageVersion, LATEST_STABLE_COMPILER_VERSION,
    LATEST_STABLE_LANGUAGE_VERSION,
};
use move_package::source_package::std_lib::StdVersion;
use serde::{Deserialize, Serialize};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    cmp::max,
    collections::BTreeMap,
    convert::TryFrom,
    fmt::{Debug, Display, Formatter},
    fs::OpenOptions,
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub const USER_AGENT: &str = concat!("aptos-cli/", env!("CARGO_PKG_VERSION"));
pub const US_IN_SECS: u64 = 1_000_000;
pub const ACCEPTED_CLOCK_SKEW_US: u64 = 5 * US_IN_SECS;
pub const DEFAULT_EXPIRATION_SECS: u64 = 30;
pub const DEFAULT_PROFILE: &str = "default";
pub const GIT_IGNORE: &str = ".gitignore";

pub const APTOS_FOLDER_GIT_IGNORE: &str = indoc! {"
    *
    testnet/
    config.yaml
"};
pub const MOVE_FOLDER_GIT_IGNORE: &str = indoc! {"
  .aptos/
  build/
  .coverage_map.mvcov
  .trace"
};

// Custom header value to identify the client
const X_APTOS_CLIENT_VALUE: &str = concat!("aptos-cli/", env!("CARGO_PKG_VERSION"));

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
    #[error(
        "The package is larger than {1} bytes ({0} bytes)! \
        To lower the size you may want to include less artifacts via `--included-artifacts`. \
        You can also override this check with `--override-size-check`. \
        Alternatively, you can use the `--chunked-publish` to enable chunked publish mode, \
        which chunks down the package and deploys it in several stages."
    )]
    PackageSizeExceeded(usize, usize),
    #[error("Unable to parse '{0}': error: {1}")]
    UnableToParse(&'static str, String),
    #[error("Unable to read file '{0}', error: {1}")]
    UnableToReadFile(String, String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
    #[error("Simulation failed with status: {0}")]
    SimulationError(String),
    #[error("Coverage failed with status: {0}")]
    CoverageError(String),
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
            CliError::PackageSizeExceeded(_, _) => "PackageSizeExceeded",
            CliError::UnableToParse(_, _) => "UnableToParse",
            CliError::UnableToReadFile(_, _) => "UnableToReadFile",
            CliError::UnexpectedError(_) => "UnexpectedError",
            CliError::SimulationError(_) => "SimulationError",
            CliError::CoverageError(_) => "CoverageError",
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
        CliError::UnexpectedError(format!("{:#}", e))
    }
}

impl From<bcs::Error> for CliError {
    fn from(e: bcs::Error) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<aptos_ledger::AptosLedgerError> for CliError {
    fn from(e: aptos_ledger::AptosLedgerError) -> Self {
        CliError::UnexpectedError(e.to_string())
    }
}

impl From<EncodingError> for CliError {
    fn from(e: EncodingError) -> Self {
        match e {
            EncodingError::BCS(s, e) => CliError::BCS(s, e),
            EncodingError::UnableToParse(s, e) => CliError::UnableToParse(s, e),
            EncodingError::UnableToReadFile(s, e) => CliError::UnableToReadFile(s, e),
            EncodingError::UTF8(s) => CliError::UnexpectedError(s),
        }
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
    /// Name of network being used, if setup from aptos init
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Private key for commands.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub private_key: Option<Ed25519PrivateKey>,
    /// Public key for commands
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub public_key: Option<Ed25519PublicKey>,
    /// Account for commands
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_address_str"
    )]
    pub account: Option<AccountAddress>,
    /// URL for the Aptos rest endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    /// URL for the Faucet endpoint (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
    /// Derivation path index of the account on ledger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
}

/// ProfileConfig but without the private parts
#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    pub has_private_key: bool,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub public_key: Option<Ed25519PublicKey>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_address_str"
    )]
    pub account: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
}

impl From<&ProfileConfig> for ProfileSummary {
    fn from(config: &ProfileConfig) -> Self {
        ProfileSummary {
            network: config.network,
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
        profile: Option<&str>,
        mode: ConfigSearchMode,
    ) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load(mode)?;

        // If no profile was given, use `default`
        if let Some(profile) = profile {
            if let Some(account_profile) = config.remove_profile(profile) {
                Ok(Some(account_profile))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} not found",
                    profile
                )))
            }
        } else {
            Ok(config.remove_profile(DEFAULT_PROFILE))
        }
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
        let no_dir = !aptos_folder.exists();
        create_dir_if_not_exist(aptos_folder.as_path())?;

        // If the `.aptos/` doesn't exist, we'll add a .gitignore in it to ignore the config file
        // so people don't save their credentials...
        if no_dir {
            write_to_user_only_file(
                aptos_folder.join(GIT_IGNORE).as_path(),
                GIT_IGNORE,
                APTOS_FOLDER_GIT_IGNORE.as_bytes(),
            )?;
        }

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
#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum KeyType {
    /// Ed25519 key used for signing
    Ed25519,
    /// X25519 key used for network handshakes and identity
    X25519,
    /// A BLS12381 key for consensus
    Bls12381,
}

impl Display for KeyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            KeyType::Ed25519 => "ed25519",
            KeyType::X25519 => "x25519",
            KeyType::Bls12381 => "bls12381",
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
            "bls12381" => Ok(KeyType::Bls12381),
            _ => Err("Invalid key type: Must be one of [ed25519, x25519]"),
        }
    }
}

#[derive(Debug, Default, Parser)]
pub struct ProfileOptions {
    /// Profile to use from the CLI config
    ///
    /// This will be used to override associated settings such as
    /// the REST URL, the Faucet URL, and the private key arguments.
    ///
    /// Defaults to "default"
    #[clap(long)]
    pub profile: Option<String>,
}

impl ProfileOptions {
    pub fn account_address(&self) -> CliTypedResult<AccountAddress> {
        let profile = self.profile()?;
        if let Some(account) = profile.account {
            return Ok(account);
        }

        Err(CliError::ConfigNotFoundError(
            self.profile
                .clone()
                .unwrap_or_else(|| DEFAULT_PROFILE.to_string()),
        ))
    }

    pub fn derivation_path(&self) -> CliTypedResult<Option<String>> {
        let profile = self.profile()?;
        Ok(profile.derivation_path)
    }

    pub fn public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        let profile = self.profile()?;
        if let Some(public_key) = profile.public_key {
            return Ok(public_key);
        }

        Err(CliError::ConfigNotFoundError(
            self.profile
                .clone()
                .unwrap_or_else(|| DEFAULT_PROFILE.to_string()),
        ))
    }

    pub fn profile_name(&self) -> Option<&str> {
        self.profile.as_ref().map(|inner| inner.trim())
    }

    pub fn profile(&self) -> CliTypedResult<ProfileConfig> {
        if let Some(profile) =
            CliConfig::load_profile(self.profile_name(), ConfigSearchMode::CurrentDirAndParents)?
        {
            return Ok(profile);
        }

        Err(CliError::ConfigNotFoundError(
            self.profile
                .clone()
                .unwrap_or_else(|| DEFAULT_PROFILE.to_string()),
        ))
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

    pub fn from_string_seed(str: &str) -> RngArgs {
        assert!(str.len() < 32);

        let mut seed = [0u8; 32];
        for (i, byte) in str.bytes().enumerate() {
            seed[i] = byte;
        }

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

/// An insertable option for use with prompts.
#[derive(Clone, Copy, Debug, Default, Parser, PartialEq, Eq)]
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
#[derive(Debug, Default, Parser, Clone, Copy)]
pub struct EncodingOptions {
    /// Encoding of data as one of [base64, bcs, hex]
    #[clap(long, default_value_t = EncodingType::Hex)]
    pub encoding: EncodingType,
}

#[derive(Debug, Parser)]
pub struct AuthenticationKeyInputOptions {
    /// Authentication Key file input
    #[clap(long, group = "authentication_key_input", value_parser)]
    auth_key_file: Option<PathBuf>,

    /// Authentication key input
    #[clap(long, group = "authentication_key_input")]
    auth_key: Option<String>,
}

impl AuthenticationKeyInputOptions {
    pub fn extract_auth_key(
        &self,
        encoding: EncodingType,
    ) -> CliTypedResult<Option<AuthenticationKey>> {
        if let Some(ref file) = self.auth_key_file {
            Ok(Some(encoding.load_key("--auth-key-file", file.as_path())?))
        } else if let Some(ref key) = self.auth_key {
            let key = key.as_bytes().to_vec();
            Ok(Some(encoding.decode_key("--auth-key", key)?))
        } else {
            Ok(None)
        }
    }

    pub fn from_public_key(key: &Ed25519PublicKey) -> AuthenticationKeyInputOptions {
        let auth_key = AuthenticationKey::ed25519(key);
        AuthenticationKeyInputOptions {
            auth_key: Some(auth_key.to_encoded_string().unwrap()),
            auth_key_file: None,
        }
    }
}

#[derive(Debug, Parser)]
pub struct PublicKeyInputOptions {
    /// Ed25519 Public key input file name
    ///
    /// Mutually exclusive with `--public-key`
    #[clap(long, group = "public_key_input", value_parser)]
    public_key_file: Option<PathBuf>,
    /// Ed25519 Public key encoded in a type as shown in `encoding`
    ///
    /// Mutually exclusive with `--public-key-file`
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

impl ExtractEd25519PublicKey for PublicKeyInputOptions {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
    ) -> CliTypedResult<Ed25519PublicKey> {
        if let Some(ref file) = self.public_key_file {
            Ok(encoding.load_key("--public-key-file", file.as_path())?)
        } else if let Some(ref key) = self.public_key {
            let key = key.as_bytes().to_vec();
            Ok(encoding.decode_key("--public-key", key)?)
        } else if let Some(Some(public_key)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
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

pub trait ParseEd25519PrivateKey {
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
            let key = strip_private_key_prefix(key)?.as_bytes().to_vec();
            Ok(Some(encoding.decode_key("--private-key", key)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Default, Parser)]
pub struct HardwareWalletOptions {
    /// BIP44 derivation path of hardware wallet account, e.g. `m/44'/637'/0'/0'/0'`
    ///
    /// Note you may need to escape single quotes in your shell, for example
    /// `m/44'/637'/0'/0'/0'` would be `m/44\'/637\'/0\'/0\'/0\'`
    #[clap(long, conflicts_with = "derivation_index")]
    pub derivation_path: Option<String>,

    /// BIP44 account index of hardware wallet account, e.g. `0`
    ///
    /// Given index `n` maps to BIP44 derivation path `m/44'/637'/n'/0'/0`
    #[clap(long, conflicts_with = "derivation_path")]
    pub derivation_index: Option<String>,
}

impl HardwareWalletOptions {
    pub fn extract_derivation_path(&self) -> CliTypedResult<Option<String>> {
        if let Some(derivation_path) = &self.derivation_path {
            Ok(Some(derivation_path.clone()))
        } else if let Some(derivation_index) = &self.derivation_index {
            let derivation_path = format!("m/44'/637'/{}'/0'/0'", derivation_index);
            Ok(Some(derivation_path))
        } else {
            Ok(None)
        }
    }

    pub fn is_hardware_wallet(&self) -> bool {
        self.derivation_path.is_some() || self.derivation_index.is_some()
    }
}

#[derive(Debug, Default, Parser)]
pub struct PrivateKeyInputOptions {
    /// Signing Ed25519 private key file path
    ///
    /// Encoded with type from `--encoding`
    /// Mutually exclusive with `--private-key`
    #[clap(long, group = "private_key_input", value_parser)]
    private_key_file: Option<PathBuf>,
    /// Signing Ed25519 private key
    ///
    /// Encoded with type from `--encoding`
    /// Mutually exclusive with `--private-key-file`
    #[clap(long, group = "private_key_input")]
    private_key: Option<String>,
}

impl ParseEd25519PrivateKey for PrivateKeyInputOptions {}

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

    pub fn has_key_or_file(&self) -> bool {
        self.private_key.is_some() || self.private_key_file.is_some()
    }

    /// Extract public key from CLI args with fallback to config
    /// This will first try to extract public key from private_key from CLI args
    /// With fallback to profile
    /// NOTE: Use this function instead of 'extract_private_key_and_address' if this is HardwareWallet profile
    /// HardwareWallet profile does not have private key in config
    pub fn extract_ed25519_public_key_and_address(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
        maybe_address: Option<AccountAddress>,
    ) -> CliTypedResult<(Ed25519PublicKey, AccountAddress)> {
        // Order of operations
        // 1. CLI inputs
        // 2. Profile
        // 3. Derived
        if let Some(private_key) = self.extract_private_key_cli(encoding)? {
            // If we use the CLI inputs, then we should derive or use the address from the input
            if let Some(address) = maybe_address {
                Ok((private_key.public_key(), address))
            } else {
                let address = account_address_from_public_key(&private_key.public_key());
                Ok((private_key.public_key(), address))
            }
        } else if let Some((Some(public_key), maybe_config_address)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| (p.public_key, p.account))
        {
            match (maybe_address, maybe_config_address) {
                (Some(address), _) => Ok((public_key, address)),
                (_, Some(address)) => Ok((public_key, address)),
                (None, None) => {
                    let address = account_address_from_public_key(&public_key);
                    Ok((public_key, address))
                },
            }
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'], or ['public_key'] must present in profile".to_string(),
            ))
        }
    }

    /// Extract address
    pub fn extract_address(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
        maybe_address: Option<AccountAddress>,
    ) -> CliTypedResult<AccountAddress> {
        // Order of operations
        // 1. CLI inputs
        // 2. Profile
        // 3. Derived
        if let Some(address) = maybe_address {
            return Ok(address);
        }

        if let Some(private_key) = self.extract_private_key_cli(encoding)? {
            // If we use the CLI inputs, then we should derive or use the address from the input
            let address = account_address_from_public_key(&private_key.public_key());
            Ok(address)
        } else if let Some((Some(public_key), maybe_config_address)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| (p.public_key, p.account))
        {
            if let Some(address) = maybe_config_address {
                Ok(address)
            } else {
                let address = account_address_from_public_key(&public_key);
                Ok(address)
            }
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'], or ['public_key'] must present in profile".to_string(),
            ))
        }
    }

    /// Extract private key from CLI args with fallback to config
    pub fn extract_private_key_and_address(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
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
        } else if let Some((Some(key), maybe_config_address)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| (p.private_key, p.account))
        {
            match (maybe_address, maybe_config_address) {
                (Some(address), _) => Ok((key, address)),
                (_, Some(address)) => Ok((key, address)),
                (None, None) => {
                    let address = account_address_from_public_key(&key.public_key());
                    Ok((key, address))
                },
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
        profile: &ProfileOptions,
    ) -> CliTypedResult<Ed25519PrivateKey> {
        if let Some(key) = self.extract_private_key_cli(encoding)? {
            Ok(key)
        } else if let Some(Some(private_key)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
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

    pub fn extract_private_key_input_from_cli_args(&self) -> CliTypedResult<Vec<u8>> {
        if let Some(ref file) = self.private_key_file {
            read_from_file(file)
        } else if let Some(ref key) = self.private_key {
            Ok(strip_private_key_prefix(key)?.as_bytes().to_vec())
        } else {
            Err(CliError::CommandArgumentError(
                "No --private-key or --private-key-file provided".to_string(),
            ))
        }
    }
}

// Extract the public key by deriving private key, fall back to public key from profile
// Order of operations
// 1. Get the private key (either from CLI input or profile), and derive the public key from it
// 2. Else get the public key directly from the config profile
// 3. Else error
impl ExtractEd25519PublicKey for PrivateKeyInputOptions {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
    ) -> CliTypedResult<Ed25519PublicKey> {
        // 1. Get the private key, and derive the public key
        let private_key = if let Some(key) = self.extract_private_key_cli(encoding)? {
            Some(key)
        } else if let Some(Some(private_key)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| p.private_key)
        {
            Some(private_key)
        } else {
            None
        };

        // 2. Get the public key from the config profile
        // 3. Else error
        if let Some(key) = private_key {
            Ok(key.public_key())
        } else if let Some(Some(public_key)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| p.public_key)
        {
            Ok(public_key)
        } else {
            Err(CliError::CommandArgumentError(
                "Unable to extract public key from Private Key input nor Profile".to_string(),
            ))
        }
    }
}

pub trait ExtractEd25519PublicKey {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
    ) -> CliTypedResult<Ed25519PublicKey>;
}

pub fn account_address_from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    let auth_key = AuthenticationKey::ed25519(public_key);
    account_address_from_auth_key(&auth_key)
}

pub fn account_address_from_auth_key(auth_key: &AuthenticationKey) -> AccountAddress {
    AccountAddress::new(*auth_key.account_address())
}

#[derive(Debug, Parser, Clone)]
pub struct SaveFile {
    /// Output file path
    #[clap(long, value_parser)]
    pub output_file: PathBuf,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl SaveFile {
    /// Check if the `output_file` exists already
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
#[derive(Debug, Parser)]
pub struct RestOptions {
    /// URL to a fullnode on the network
    ///
    /// Defaults to the URL in the `default` profile
    #[clap(long)]
    pub(crate) url: Option<reqwest::Url>,

    /// Connection timeout in seconds, used for the REST endpoint of the fullnode
    #[clap(long, default_value_t = DEFAULT_EXPIRATION_SECS, alias = "connection-timeout-s")]
    pub connection_timeout_secs: u64,

    /// Key to use for ratelimiting purposes with the node API. This value will be used
    /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
    /// environment variable.
    #[clap(long, env)]
    pub node_api_key: Option<String>,
}

impl Default for RestOptions {
    fn default() -> Self {
        Self {
            url: None,
            connection_timeout_secs: DEFAULT_EXPIRATION_SECS,
            node_api_key: None,
        }
    }
}

impl RestOptions {
    pub fn new(url: Option<reqwest::Url>, connection_timeout_secs: Option<u64>) -> Self {
        RestOptions {
            url,
            connection_timeout_secs: connection_timeout_secs.unwrap_or(DEFAULT_EXPIRATION_SECS),
            node_api_key: None,
        }
    }

    /// Retrieve the URL from the profile or the command line
    pub fn url(&self, profile: &ProfileOptions) -> CliTypedResult<reqwest::Url> {
        if let Some(ref url) = self.url {
            Ok(url.clone())
        } else if let Some(Some(url)) = CliConfig::load_profile(
            profile.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| p.rest_url)
        {
            reqwest::Url::parse(&url)
                .map_err(|err| CliError::UnableToParse("Rest URL", err.to_string()))
        } else {
            Err(CliError::CommandArgumentError("No rest url given.  Please add --url or add a rest_url to the .aptos/config.yaml for the current profile".to_string()))
        }
    }

    pub fn client(&self, profile: &ProfileOptions) -> CliTypedResult<Client> {
        let mut client = Client::builder(AptosBaseUrl::Custom(self.url(profile)?))
            .timeout(Duration::from_secs(self.connection_timeout_secs))
            .header(aptos_api_types::X_APTOS_CLIENT, X_APTOS_CLIENT_VALUE)?;
        if let Some(node_api_key) = &self.node_api_key {
            client = client.api_key(node_api_key)?;
        }
        Ok(client.build())
    }
}

/// Options for optimization level
#[derive(Debug, Clone, Parser)]
pub enum OptimizationLevel {
    /// No optimizations
    None,
    /// Default optimization level
    Default,
    /// Extra optimizations, that may take more time
    Extra,
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::Default
    }
}

impl FromStr for OptimizationLevel {
    type Err = anyhow::Error;

    /// Parses an optimization level, or default.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(Self::None),
            "" | "default" => Ok(Self::Default),
            "extra" => Ok(Self::Extra),
            _ => bail!(
                "unrecognized optimization level `{}` (supported versions: `none`, `default`, `extra`)",
                s
            ),
        }
    }
}

/// Options for compiling a move package.
#[derive(Debug, Clone, Parser)]
pub struct MovePackageOptions {
    /// Path to a move package (the folder with a Move.toml file).  Defaults to current directory.
    #[clap(long, value_parser)]
    pub package_dir: Option<PathBuf>,

    /// Path to save the compiled move package
    ///
    /// Defaults to `<package_dir>/build`
    #[clap(long, value_parser)]
    pub output_dir: Option<PathBuf>,

    /// Named addresses for the move binary
    ///
    /// Example: alice=0x1234, bob=0x5678
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, value_parser = crate::common::utils::parse_map::<String, AccountAddressWrapper>, default_value = "")]
    pub(crate) named_addresses: BTreeMap<String, AccountAddressWrapper>,

    /// Override the standard library version by mainnet/testnet/devnet
    #[clap(long, value_parser)]
    pub override_std: Option<StdVersion>,

    /// Skip pulling the latest git dependencies
    ///
    /// If you don't have a network connection, the compiler may fail due
    /// to no ability to pull git dependencies.  This will allow overriding
    /// this for local development.
    #[clap(long)]
    pub(crate) skip_fetch_latest_git_deps: bool,

    /// Do not complain about unknown attributes in Move code.
    #[clap(long)]
    pub skip_attribute_checks: bool,

    /// Enables dev mode, which uses all dev-addresses and dev-dependencies
    ///
    /// Dev mode allows for changing dependencies and addresses to the preset [dev-addresses] and
    /// [dev-dependencies] fields.  This works both inside and out of tests for using preset values.
    ///
    /// Currently, it also additionally pulls in all test compilation artifacts
    #[clap(long)]
    pub dev: bool,

    /// Skip extended checks (such as checks for the #[view] attribute) on test code.
    #[clap(long, default_value = "false")]
    pub skip_checks_on_test_code: bool,

    /// Select optimization level.  Choices are "none", "default", or "extra".
    /// Level "extra" may spend more time on expensive optimizations in the future.
    /// Level "none" does no optimizations, possibly leading to use of too many runtime resources.
    /// Level "default" is the recommended level, and the default if not provided.
    #[clap(long, alias = "optimization_level", value_parser = clap::value_parser!(OptimizationLevel))]
    pub optimize: Option<OptimizationLevel>,

    /// Experiments
    #[clap(long, hide(true))]
    pub experiments: Vec<String>,

    /// ...or --bytecode BYTECODE_VERSION
    /// Specify the version of the bytecode the compiler is going to emit.
    /// If not provided, it is inferred from the language version.
    #[clap(long, alias = "bytecode", verbatim_doc_comment)]
    pub bytecode_version: Option<u32>,

    /// ...or --compiler COMPILER_VERSION
    /// Specify the version of the compiler (must be at least 2).
    /// Defaults to the latest stable compiler version.
    #[clap(long, value_parser = clap::value_parser!(CompilerVersion),
           alias = "compiler",
           default_value = LATEST_STABLE_COMPILER_VERSION,
           verbatim_doc_comment)]
    pub compiler_version: Option<CompilerVersion>,

    /// ...or --language LANGUAGE_VERSION
    /// Specify the language version to be supported.
    /// Defaults to the latest stable language version.
    #[clap(long, value_parser = clap::value_parser!(LanguageVersion),
           alias = "language",
           default_value = LATEST_STABLE_LANGUAGE_VERSION,
           verbatim_doc_comment)]
    pub language_version: Option<LanguageVersion>,

    /// Fail the compilation if there are any warnings.
    #[clap(long)]
    pub fail_on_warning: bool,
}

impl Default for MovePackageOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MovePackageOptions {
    pub fn new() -> Self {
        Self {
            dev: false,
            package_dir: None,
            output_dir: None,
            named_addresses: Default::default(),
            override_std: None,
            skip_fetch_latest_git_deps: true,
            bytecode_version: None,
            compiler_version: Some(CompilerVersion::latest_stable()),
            language_version: Some(LanguageVersion::latest_stable()),
            skip_attribute_checks: false,
            skip_checks_on_test_code: false,
            optimize: None,
            fail_on_warning: false,
            experiments: vec![],
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

    pub fn add_named_address(&mut self, key: String, value: String) {
        self.named_addresses
            .insert(key, AccountAddressWrapper::from_str(&value).unwrap());
    }

    /// Compute the experiments to be used for the compiler.
    pub fn compute_experiments(&self) -> Vec<String> {
        let mut experiments = self.experiments.clone();
        let mut set = |k: &str, v: bool| {
            experiments.push(format!("{}={}", k, if v { "on" } else { "off" }));
        };
        match self.optimize {
            None | Some(OptimizationLevel::Default) => {
                set(Experiment::OPTIMIZE, true);
            },
            Some(OptimizationLevel::None) => {
                set(Experiment::OPTIMIZE, false);
            },
            Some(OptimizationLevel::Extra) => {
                set(Experiment::OPTIMIZE_EXTRA, true);
                set(Experiment::OPTIMIZE, true);
            },
        }
        if self.fail_on_warning {
            set(Experiment::FAIL_ON_WARNING, true);
        }
        experiments
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
    if let Ok(account_address) = AccountAddress::from_str(str) {
        Ok(account_address)
    } else if let Some(Some(account_address)) =
        CliConfig::load_profile(Some(str), ConfigSearchMode::CurrentDirAndParents)?
            .map(|p| p.account)
    {
        Ok(account_address)
    } else if let Some(Some(private_key)) =
        CliConfig::load_profile(Some(str), ConfigSearchMode::CurrentDirAndParents)?
            .map(|p| p.private_key)
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
    } else if let Ok(account_address) = AccountAddress::from_str(str) {
        Ok(Some(account_address))
    } else if let Some(Some(private_key)) =
        CliConfig::load_profile(Some(str), ConfigSearchMode::CurrentDirAndParents)?
            .map(|p| p.private_key)
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

    /// Returns whether the error should be JSONifyed.
    fn jsonify_error_output(&self) -> bool {
        true
    }

    /// Executes the command, returning a command specific type
    async fn execute(self) -> CliTypedResult<T>;

    /// Executes the command, and serializes it to the common JSON output type
    async fn execute_serialized(self) -> CliResult {
        self.execute_serialized_with_logging_level(Level::Warn)
            .await
    }

    /// Execute the command with customized logging level
    async fn execute_serialized_with_logging_level(self, level: Level) -> CliResult {
        let command_name = self.command_name();
        start_logger(level);
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        to_common_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
    }

    /// Same as execute serialized without setting up logging
    async fn execute_serialized_without_logger(self) -> CliResult {
        let command_name = self.command_name();
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        to_common_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
    }

    /// Executes the command, and throws away Ok(result) for the string Success
    async fn execute_serialized_success(self) -> CliResult {
        start_logger(Level::Warn);
        let command_name = self.command_name();
        let start_time = Instant::now();
        let jsonify_error_output = self.jsonify_error_output();
        to_common_success_result(
            command_name,
            start_time,
            self.execute().await,
            jsonify_error_output,
        )
        .await
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
    // Question[Orderless]: Is it backward compatible to replace sequence_number with replay_protector?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay_protector: Option<ReplayProtector>,
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
                replay_protector: Some(txn.request.replay_protector()),
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
                replay_protector: Some(txn.request.replay_protector()),
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
                replay_protector: None,
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
                replay_protector: None,
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
                replay_protector: None,
            },
            Transaction::BlockEpilogueTransaction(txn) => TransactionSummary {
                transaction_hash: txn.info.hash,
                success: Some(txn.info.success),
                version: Some(txn.info.version.0),
                vm_status: Some(txn.info.vm_status.clone()),
                timestamp_us: Some(txn.timestamp.0),
                sender: None,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                replay_protector: None,
            },
            Transaction::ValidatorTransaction(txn) => TransactionSummary {
                transaction_hash: txn.transaction_info().hash,
                gas_used: None,
                gas_unit_price: None,
                pending: None,
                sender: None,
                replay_protector: None,
                success: Some(txn.transaction_info().success),
                timestamp_us: Some(txn.timestamp().0),
                version: Some(txn.transaction_info().version.0),
                vm_status: Some(txn.transaction_info().vm_status.clone()),
            },
        }
    }
}

/// A summary of a `WriteSetChange` for easy printing
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
    /// URL for the faucet endpoint e.g. `https://faucet.devnet.aptoslabs.com`
    #[clap(long)]
    pub faucet_url: Option<reqwest::Url>,

    /// Auth token to bypass faucet ratelimits. You can also set this as an environment
    /// variable with FAUCET_AUTH_TOKEN.
    #[clap(long, env)]
    pub faucet_auth_token: Option<String>,
}

impl FaucetOptions {
    pub fn new(faucet_url: Option<reqwest::Url>, faucet_auth_token: Option<String>) -> Self {
        FaucetOptions {
            faucet_url,
            faucet_auth_token,
        }
    }

    fn faucet_url(&self, profile_options: &ProfileOptions) -> CliTypedResult<reqwest::Url> {
        if let Some(ref faucet_url) = self.faucet_url {
            return Ok(faucet_url.clone());
        }
        let profile = CliConfig::load_profile(
            profile_options.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?;
        let profile = match profile {
            Some(profile) => profile,
            None => {
                return Err(CliError::CommandArgumentError(format!(
                    "Profile \"{}\" not found.",
                    profile_options.profile_name().unwrap_or(DEFAULT_PROFILE)
                )))
            },
        };

        match profile.faucet_url {
            Some(url) => reqwest::Url::parse(&url)
                .map_err(|err| CliError::UnableToParse("config faucet_url", err.to_string())),
            None => match profile.network {
                Some(Network::Mainnet) => {
                    Err(CliError::CommandArgumentError("There is no faucet for mainnet. Please create and fund the account by transferring funds from another account. If you are confident you want to use a faucet, set --faucet-url or add a faucet URL to .aptos/config.yaml for the current profile".to_string()))
                },
                Some(Network::Testnet) => {
                    Err(CliError::CommandArgumentError(format!("To get testnet APT you must visit {}. If you are confident you want to use a faucet programmatically, set --faucet-url or add a faucet URL to .aptos/config.yaml for the current profile", get_mint_site_url(None))))
                },
                _ => {
                    Err(CliError::CommandArgumentError("No faucet given. Please set --faucet-url or add a faucet URL to .aptos/config.yaml for the current profile".to_string()))
                },
            },
        }
    }

    /// Fund an account with the faucet.
    pub async fn fund_account(
        &self,
        rest_client: Client,
        profile: &ProfileOptions,
        num_octas: u64,
        address: AccountAddress,
    ) -> CliTypedResult<()> {
        fund_account(
            rest_client,
            self.faucet_url(profile)?,
            self.faucet_auth_token.as_deref(),
            address,
            num_octas,
        )
        .await
    }
}

/// Gas price options for manipulating how to prioritize transactions
#[derive(Debug, Clone, Eq, Parser, PartialEq)]
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
    /// Number of seconds to expire the transaction
    ///
    /// This is the number of seconds from the current local computer time.
    #[clap(long, default_value_t = DEFAULT_EXPIRATION_SECS)]
    pub expiration_secs: u64,
}

impl Default for GasOptions {
    fn default() -> Self {
        GasOptions {
            gas_unit_price: None,
            max_gas: None,
            expiration_secs: DEFAULT_EXPIRATION_SECS,
        }
    }
}

#[derive(Debug)]
pub enum AccountType {
    Local,
    HardwareWallet,
}

/// Common options for interacting with an account for a validator
#[derive(Debug, Default, Parser)]
pub struct TransactionOptions {
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
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
    pub prompt_options: PromptOptions,

    /// If this option is set, simulate the transaction locally.
    #[clap(long)]
    pub(crate) local: bool,

    /// If this option is set, benchmark the transaction locally.
    #[clap(long)]
    pub(crate) benchmark: bool,

    /// If this option is set, simulate the transaction locally using the debugger and generate
    /// flamegraphs that reflect the gas usage.
    #[clap(long)]
    pub(crate) profile_gas: bool,

    /// Replay protection mechanism to use when generating the transaction.
    ///
    /// When "turbo" is chosen, the transaction will contain a replay protection nonce.
    ///
    /// When "seqnum" is chosen, the transaction will contain a sequence number that matches with the sender's onchain sequence number.
    #[clap(long, default_value_t = ReplayProtectionType::Seqnum)]
    pub(crate) replay_protection_type: ReplayProtectionType,
}

impl TransactionOptions {
    /// Builds a rest client
    pub fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options)
    }

    pub fn get_transaction_account_type(&self) -> CliTypedResult<AccountType> {
        if self.private_key_options.private_key.is_some()
            || self.private_key_options.private_key_file.is_some()
        {
            Ok(AccountType::Local)
        } else if let Some(profile) = CliConfig::load_profile(
            self.profile_options.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )? {
            if profile.private_key.is_some() {
                Ok(AccountType::Local)
            } else {
                Ok(AccountType::HardwareWallet)
            }
        } else {
            Err(CliError::CommandArgumentError(
                "One of ['--private-key', '--private-key-file'] or profile must be used"
                    .to_string(),
            ))
        }
    }

    /// Retrieves the private key and the associated address
    /// TODO: Cache this information
    pub fn get_key_and_address(&self) -> CliTypedResult<(Ed25519PrivateKey, AccountAddress)> {
        self.private_key_options.extract_private_key_and_address(
            self.encoding_options.encoding,
            &self.profile_options,
            self.sender_account,
        )
    }

    pub fn get_public_key_and_address(&self) -> CliTypedResult<(Ed25519PublicKey, AccountAddress)> {
        self.private_key_options
            .extract_ed25519_public_key_and_address(
                self.encoding_options.encoding,
                &self.profile_options,
                self.sender_account,
            )
    }

    pub fn sender_address(&self) -> CliTypedResult<AccountAddress> {
        Ok(self.get_key_and_address()?.1)
    }

    pub fn get_public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        self.private_key_options
            .extract_public_key(self.encoding_options.encoding, &self.profile_options)
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

    pub async fn view(&self, payload: ViewFunction) -> CliTypedResult<Vec<serde_json::Value>> {
        let client = self.rest_client()?;
        Ok(client
            .view_bcs_with_json_response(&payload, None)
            .await?
            .into_inner())
    }

    /// Submit a transaction
    pub async fn submit_transaction(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<Transaction> {
        let client = self.rest_client()?;
        let (sender_public_key, sender_address) = self.get_public_key_and_address()?;

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

        // Get sequence number for account
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let sequence_number = account.sequence_number;

        // Retrieve local time, and ensure it's within an expected skew of the blockchain
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?
            .as_secs();
        let now_usecs = now * US_IN_SECS;

        // Warn local user that clock is skewed behind the blockchain.
        // There will always be a little lag from real time to blockchain time
        if now_usecs < state.timestamp_usecs - ACCEPTED_CLOCK_SKEW_US {
            eprintln!("Local clock is is skewed from blockchain clock.  Clock is more than {} seconds behind the blockchain {}", ACCEPTED_CLOCK_SKEW_US, state.timestamp_usecs / US_IN_SECS );
        }
        let expiration_time_secs = now + self.gas_options.expiration_secs;

        let chain_id = ChainId::new(state.chain_id);
        // TODO: Check auth key against current private key and provide a better message

        let max_gas = if let Some(max_gas) = self.gas_options.max_gas {
            // If the gas unit price was estimated ask, but otherwise you've chosen hwo much you want to spend
            if ask_to_confirm_price {
                let message = format!("Do you want to submit transaction for a maximum of {} Octas at a gas unit price of {} Octas?",  max_gas * gas_unit_price, gas_unit_price);
                prompt_yes_with_override(&message, self.prompt_options)?;
            }
            max_gas
        } else {
            let transaction_factory =
                TransactionFactory::new(chain_id).with_gas_unit_price(gas_unit_price);

            let txn_builder = transaction_factory
                .payload(payload.clone())
                .sender(sender_address)
                .sequence_number(sequence_number)
                .expiration_timestamp_secs(expiration_time_secs);

            let unsigned_transaction = if self.replay_protection_type == ReplayProtectionType::Turbo
            {
                txn_builder.upgrade_payload(true, true).build()
            } else {
                txn_builder.build()
            };

            let signed_transaction = SignedTransaction::new(
                unsigned_transaction,
                sender_public_key.clone(),
                Ed25519Signature::try_from([0u8; 64].as_ref()).unwrap(),
            );

            let txns = client
                .simulate_with_gas_estimation(&signed_transaction, true, false)
                .await?
                .into_inner();
            let simulated_txn = txns.first().unwrap();

            // Check if the transaction will pass, if it doesn't then fail
            if !simulated_txn.info.success {
                return Err(CliError::SimulationError(
                    simulated_txn.info.vm_status.clone(),
                ));
            }

            // Take the gas used and use a headroom factor on it
            let gas_used = simulated_txn.info.gas_used.0;
            // TODO: remove the hardcoded 530 as it's the minumum gas units required for the transaction that will
            // automatically create an account for stateless account.
            let adjusted_max_gas =
                adjust_gas_headroom(gas_used, max(simulated_txn.request.max_gas_amount.0, 530));

            // Ask if you want to accept the estimate amount
            let upper_cost_bound = adjusted_max_gas * gas_unit_price;
            let lower_cost_bound = gas_used * gas_unit_price;
            let message = format!(
                    "Do you want to submit a transaction for a range of [{} - {}] Octas at a gas unit price of {} Octas?",
                    lower_cost_bound,
                    upper_cost_bound,
                    gas_unit_price);
            prompt_yes_with_override(&message, self.prompt_options)?;
            adjusted_max_gas
        };

        // Build a transaction
        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.gas_options.expiration_secs);

        // Sign it with the appropriate signer
        let transaction = match self.get_transaction_account_type() {
            Ok(AccountType::Local) => {
                let (private_key, _) = self.get_key_and_address()?;
                let sender_account =
                    &mut LocalAccount::new(sender_address, private_key, sequence_number);
                let mut txn_builder = transaction_factory.payload(payload);
                if self.replay_protection_type == ReplayProtectionType::Turbo {
                    txn_builder = txn_builder.upgrade_payload(true, true);
                };
                sender_account.sign_with_transaction_builder(txn_builder)
            },
            Ok(AccountType::HardwareWallet) => {
                let sender_account = &mut HardwareWalletAccount::new(
                    sender_address,
                    sender_public_key,
                    self.profile_options
                        .derivation_path()
                        .expect("derivative path is missing from profile")
                        .unwrap(),
                    HardwareWalletType::Ledger,
                    sequence_number,
                );
                let mut txn_builder = transaction_factory.payload(payload);
                if self.replay_protection_type == ReplayProtectionType::Turbo {
                    txn_builder = txn_builder.upgrade_payload(true, true);
                };
                sender_account.sign_with_transaction_builder(txn_builder)?
            },
            Err(err) => return Err(err),
        };

        // Submit the transaction, printing out a useful transaction link
        client
            .submit_bcs(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;
        let transaction_hash = transaction.clone().committed_hash();
        let network = self.profile_options.profile().ok().and_then(|profile| {
            if let Some(network) = profile.network {
                Some(network)
            } else {
                // Approximate network from URL
                match profile.rest_url {
                    None => None,
                    Some(url) => {
                        if url.contains("mainnet") {
                            Some(Network::Mainnet)
                        } else if url.contains("testnet") {
                            Some(Network::Testnet)
                        } else if url.contains("devnet") {
                            Some(Network::Devnet)
                        } else if url.contains("localhost") || url.contains("127.0.0.1") {
                            Some(Network::Local)
                        } else {
                            None
                        }
                    },
                }
            }
        });
        eprintln!(
            "Transaction submitted: {}",
            explorer_transaction_link(transaction_hash, network)
        );
        let response = client
            .wait_for_signed_transaction(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;

        Ok(response.into_inner())
    }

    /// Simulates a transaction locally, using the debugger to fetch required data from remote.
    async fn simulate_using_debugger<F>(
        &self,
        payload: TransactionPayload,
        execute: F,
    ) -> CliTypedResult<TransactionSummary>
    where
        F: FnOnce(
            &AptosDebugger,
            u64,
            SignedTransaction,
            aptos_crypto::HashValue,
        ) -> CliTypedResult<(VMStatus, VMOutput)>,
    {
        let client = self.rest_client()?;

        // Fetch the chain states required for the simulation
        // TODO(Gas): get the following from the chain
        const DEFAULT_GAS_UNIT_PRICE: u64 = 100;
        const DEFAULT_MAX_GAS: u64 = 2_000_000;

        let (sender_key, sender_address) = self.get_key_and_address()?;
        let gas_unit_price = self
            .gas_options
            .gas_unit_price
            .unwrap_or(DEFAULT_GAS_UNIT_PRICE);
        let (account, state) = get_account_with_state(&client, sender_address).await?;
        let version = state.version;
        let chain_id = ChainId::new(state.chain_id);
        let sequence_number = account.sequence_number;

        let balance = client
            .view_apt_account_balance_at_version(sender_address, version)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner();

        let max_gas = self.gas_options.max_gas.unwrap_or_else(|| {
            if gas_unit_price == 0 {
                DEFAULT_MAX_GAS
            } else {
                std::cmp::min(balance / gas_unit_price, DEFAULT_MAX_GAS)
            }
        });

        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(gas_unit_price)
            .with_max_gas_amount(max_gas)
            .with_transaction_expiration_time(self.gas_options.expiration_secs);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
        let hash = transaction.committed_hash();

        let debugger = AptosDebugger::rest_client(client).unwrap();
        let (vm_status, vm_output) = execute(&debugger, version, transaction, hash)?;

        let success = match vm_output.status() {
            TransactionStatus::Keep(exec_status) => Some(exec_status.is_success()),
            TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
        };

        let summary = TransactionSummary {
            transaction_hash: hash.into(),
            gas_used: Some(vm_output.gas_used()),
            gas_unit_price: Some(gas_unit_price),
            pending: None,
            sender: Some(sender_address),
            replay_protector: None, // The transaction is not comitted so there is no new sequence number.
            success,
            timestamp_us: None,
            version: Some(version), // The transaction is not comitted so there is no new version.
            vm_status: Some(vm_status.to_string()),
        };

        Ok(summary)
    }

    /// Simulates a transaction locally.
    pub async fn simulate_locally(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally...");

        self.simulate_using_debugger(payload, local_simulation::run_transaction_using_debugger)
            .await
    }

    /// Benchmarks the transaction payload locally.
    /// The transaction is executed multiple times, and the median value is calculated to improve
    /// the accuracy of the measurement results.
    pub async fn benchmark_locally(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Benchmarking transaction locally...");

        self.simulate_using_debugger(
            payload,
            local_simulation::benchmark_transaction_using_debugger,
        )
        .await
    }

    /// Simulates the transaction locally with the gas profiler enabled.
    pub async fn profile_gas(
        &self,
        payload: TransactionPayload,
    ) -> CliTypedResult<TransactionSummary> {
        println!();
        println!("Simulating transaction locally using the gas profiler...");

        self.simulate_using_debugger(
            payload,
            local_simulation::profile_transaction_using_debugger,
        )
        .await
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
    ///
    /// Defaults to the profile's `AccountAddress`
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) pool_address: Option<AccountAddress>,
}

#[derive(Parser)]
pub struct PoolAddressArgs {
    /// Address of the Staking pool
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) pool_address: AccountAddress,
}

/// Common options for interactions with a multisig account.
#[derive(Clone, Debug, Parser, Serialize)]
pub struct MultisigAccount {
    /// The address of the multisig account to interact with
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) multisig_address: AccountAddress,
}

#[derive(Clone, Debug, Parser, Serialize)]
pub struct MultisigAccountWithSequenceNumber {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    /// Multisig account sequence number to interact with
    #[clap(long)]
    pub(crate) sequence_number: u64,
}

#[derive(Debug, Default, Parser)]
pub struct TypeArgVec {
    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u16 u32 u64 u128 u256 bool address vector signer`
    #[clap(long, num_args = 0..)]
    pub(crate) type_args: Vec<MoveType>,
}

impl TryFrom<&Vec<String>> for TypeArgVec {
    type Error = CliError;

    fn try_from(value: &Vec<String>) -> Result<Self, Self::Error> {
        let mut type_args = vec![];
        for string_ref in value {
            type_args.push(
                MoveType::from_str(string_ref)
                    .map_err(|err| CliError::UnableToParse("type argument", err.to_string()))?,
            );
        }
        Ok(TypeArgVec { type_args })
    }
}

impl TryInto<Vec<TypeTag>> for TypeArgVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<TypeTag>, Self::Error> {
        let mut type_tags: Vec<TypeTag> = vec![];
        for type_arg in self.type_args.iter() {
            type_tags.push(
                TypeTag::try_from(type_arg)
                    .map_err(|err| CliError::UnableToParse("type argument", err.to_string()))?,
            );
        }
        Ok(type_tags)
    }
}

#[derive(Clone, Debug, Default, Parser)]
pub struct ArgWithTypeVec {
    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [address, bool, hex, string, u8, u16, u32, u64, u128, u256, raw]
    ///
    /// Vectors may be specified using JSON array literal syntax (you may need to escape this with
    /// quotes based on your shell interpreter)
    ///
    /// Example: `address:0x1 bool:true u8:0 u256:1234 "bool:[true, false]" 'address:[["0xace", "0xbee"], []]'`
    #[clap(long, num_args = 0..)]
    pub(crate) args: Vec<ArgWithType>,
}

impl TryFrom<&Vec<ArgWithTypeJSON>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_from(value: &Vec<ArgWithTypeJSON>) -> Result<Self, Self::Error> {
        let mut args = vec![];
        for arg_json_ref in value {
            let function_arg_type = FunctionArgType::from_str(&arg_json_ref.arg_type)?;
            args.push(function_arg_type.parse_arg_json(&arg_json_ref.value)?);
        }
        Ok(ArgWithTypeVec { args })
    }
}

impl TryInto<Vec<TransactionArgument>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<TransactionArgument>, Self::Error> {
        let mut args = vec![];
        for arg in self.args {
            args.push(
                (&arg)
                    .try_into()
                    .context(format!("Failed to parse arg {:?}", arg))
                    .map_err(|err| CliError::CommandArgumentError(err.to_string()))?,
            );
        }
        Ok(args)
    }
}

impl TryInto<Vec<Vec<u8>>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<Vec<u8>>, Self::Error> {
        Ok(self
            .args
            .into_iter()
            .map(|arg_with_type| arg_with_type.arg)
            .collect())
    }
}

impl TryInto<Vec<serde_json::Value>> for ArgWithTypeVec {
    type Error = CliError;

    fn try_into(self) -> Result<Vec<serde_json::Value>, Self::Error> {
        let mut args = vec![];
        for arg in self.args {
            args.push(arg.to_json()?);
        }
        Ok(args)
    }
}

/// Common options for constructing an entry function transaction payload.
#[derive(Debug, Parser)]
pub struct EntryFunctionArguments {
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::message::set_message`
    #[clap(long, required_unless_present = "json_file")]
    pub function_id: Option<MemberId>,

    #[clap(flatten)]
    pub(crate) type_arg_vec: TypeArgVec,
    #[clap(flatten)]
    pub(crate) arg_vec: ArgWithTypeVec,

    /// JSON file specifying public entry function ID, type arguments, and arguments.
    #[clap(long, value_parser, conflicts_with_all = &["function_id", "args", "type_args"])]
    pub(crate) json_file: Option<PathBuf>,
}

impl EntryFunctionArguments {
    /// Get instance as if all fields passed from command line, parsing JSON input file if needed.
    fn check_input_style(self) -> CliTypedResult<EntryFunctionArguments> {
        if let Some(json_path) = self.json_file {
            Ok(parse_json_file::<EntryFunctionArgumentsJSON>(&json_path)?.try_into()?)
        } else {
            Ok(self)
        }
    }
}

impl TryInto<EntryFunction> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<EntryFunction, Self::Error> {
        let entry_function_args = self.check_input_style()?;
        let function_id: MemberId = (&entry_function_args).try_into()?;
        Ok(EntryFunction::new(
            function_id.module_id,
            function_id.member_id,
            entry_function_args.type_arg_vec.try_into()?,
            entry_function_args.arg_vec.try_into()?,
        ))
    }
}

impl TryInto<ViewFunction> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<ViewFunction, Self::Error> {
        let view_function_args = self.check_input_style()?;
        let function_id: MemberId = (&view_function_args).try_into()?;
        Ok(ViewFunction {
            module: function_id.module_id,
            function: function_id.member_id,
            ty_args: view_function_args.type_arg_vec.try_into()?,
            args: view_function_args.arg_vec.try_into()?,
        })
    }
}

impl TryInto<MultisigTransactionPayload> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<MultisigTransactionPayload, Self::Error> {
        Ok(MultisigTransactionPayload::EntryFunction(self.try_into()?))
    }
}

impl TryInto<MemberId> for &EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<MemberId, Self::Error> {
        self.function_id
            .clone()
            .ok_or_else(|| CliError::CommandArgumentError("No function ID provided".to_string()))
    }
}

impl TryInto<ViewRequest> for EntryFunctionArguments {
    type Error = CliError;

    fn try_into(self) -> Result<ViewRequest, Self::Error> {
        let entry_function_args = self.check_input_style()?;
        let function_id: MemberId = (&entry_function_args).try_into()?;
        Ok(ViewRequest {
            function: EntryFunctionId {
                module: function_id.module_id.into(),
                name: function_id.member_id.into(),
            },
            type_arguments: entry_function_args.type_arg_vec.type_args,
            arguments: entry_function_args.arg_vec.try_into()?,
        })
    }
}

/// Common options for constructing a script payload
#[derive(Debug, Default, Parser)]
pub struct ScriptFunctionArguments {
    #[clap(flatten)]
    pub(crate) type_arg_vec: TypeArgVec,
    #[clap(flatten)]
    pub(crate) arg_vec: ArgWithTypeVec,

    /// JSON file specifying type arguments and arguments.
    #[clap(long, value_parser, conflicts_with_all = &["args", "type_args"])]
    pub(crate) json_file: Option<PathBuf>,
}

impl ScriptFunctionArguments {
    /// Get instance as if all fields passed from command line, parsing JSON input file if needed.
    fn check_input_style(self) -> CliTypedResult<ScriptFunctionArguments> {
        if let Some(json_path) = self.json_file {
            Ok(parse_json_file::<ScriptFunctionArgumentsJSON>(&json_path)?.try_into()?)
        } else {
            Ok(self)
        }
    }

    pub fn create_script_payload(self, bytecode: Vec<u8>) -> CliTypedResult<TransactionPayload> {
        let script_function_args = self.check_input_style()?;
        Ok(TransactionPayload::Script(Script::new(
            bytecode,
            script_function_args.type_arg_vec.try_into()?,
            script_function_args.arg_vec.try_into()?,
        )))
    }
}

#[derive(Deserialize, Serialize)]
/// JSON file format for function arguments.
pub struct ArgWithTypeJSON {
    #[serde(rename = "type")]
    pub(crate) arg_type: String,
    pub(crate) value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
/// JSON file format for entry function arguments.
pub struct EntryFunctionArgumentsJSON {
    pub(crate) function_id: String,
    pub(crate) type_args: Vec<String>,
    pub(crate) args: Vec<ArgWithTypeJSON>,
}

impl TryInto<EntryFunctionArguments> for EntryFunctionArgumentsJSON {
    type Error = CliError;

    fn try_into(self) -> Result<EntryFunctionArguments, Self::Error> {
        Ok(EntryFunctionArguments {
            function_id: Some(MemberId::from_str(&self.function_id)?),
            type_arg_vec: TypeArgVec::try_from(&self.type_args)?,
            arg_vec: ArgWithTypeVec::try_from(&self.args)?,
            json_file: None,
        })
    }
}

#[derive(Deserialize)]
/// JSON file format for script function arguments.
struct ScriptFunctionArgumentsJSON {
    type_args: Vec<String>,
    args: Vec<ArgWithTypeJSON>,
}

impl TryInto<ScriptFunctionArguments> for ScriptFunctionArgumentsJSON {
    type Error = CliError;

    fn try_into(self) -> Result<ScriptFunctionArguments, Self::Error> {
        Ok(ScriptFunctionArguments {
            type_arg_vec: TypeArgVec::try_from(&self.type_args)?,
            arg_vec: ArgWithTypeVec::try_from(&self.args)?,
            json_file: None,
        })
    }
}

#[derive(Parser)]
pub struct OverrideSizeCheckOption {
    /// Whether to override the check for maximal size of published data
    ///
    /// This won't bypass on chain checks, so if you are not allowed to go over the size check, it
    /// will still be blocked from publishing.
    #[clap(long)]
    pub(crate) override_size_check: bool,
}

#[derive(Parser)]
pub struct LargePackagesModuleOption {
    /// Address of the `large_packages` move module for chunked publishing
    ///
    /// By default, on the module is published at `0x0e1ca3011bdd07246d4d16d909dbb2d6953a86c4735d5acf5865d962c630cce7`
    /// on Testnet and Mainnet, and `0x7` on localnest/devnet.
    /// On any custom network where neither is used, you will need to first publish it from the framework
    /// under move-examples/large_packages.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) large_packages_module_address: Option<AccountAddress>,
}

impl LargePackagesModuleOption {
    pub(crate) async fn large_packages_module_address(
        &self,
        client: &Client,
    ) -> Result<AccountAddress, CliError> {
        if let Some(address) = self.large_packages_module_address {
            Ok(address)
        } else {
            let chain_id = ChainId::new(client.get_ledger_information().await?.inner().chain_id);
            Ok(
                AccountAddress::from_str_strict(default_large_packages_module_address(&chain_id))
                    .map_err(|err| {
                    CliError::UnableToParse("Default Large Package Module Address", err.to_string())
                })?,
            )
        }
    }
}

#[derive(Parser)]
pub struct ChunkedPublishOption {
    /// Whether to publish a package in a chunked mode. This may require more than one transaction
    /// for publishing the Move package.
    ///
    /// Use this option for publishing large packages exceeding `MAX_PUBLISH_PACKAGE_SIZE`.
    #[clap(long)]
    pub(crate) chunked_publish: bool,

    #[clap(flatten)]
    pub(crate) large_packages_module: LargePackagesModuleOption,

    /// Size of the code chunk in bytes for splitting bytecode and metadata of large packages
    ///
    /// By default, the chunk size is set to `CHUNK_SIZE_IN_BYTES`. A smaller chunk size will result
    /// in more transactions required to publish a package, while a larger chunk size might cause
    /// transaction to fail due to exceeding the execution gas limit.
    #[clap(long, default_value_t = CHUNK_SIZE_IN_BYTES)]
    pub(crate) chunk_size: usize,
}

/// For minting testnet APT.
pub fn get_mint_site_url(address: Option<AccountAddress>) -> String {
    let params = match address {
        Some(address) => format!("?address={}", address.to_standard_string()),
        None => "".to_string(),
    };
    format!("https://aptos.dev/network/faucet{}", params)
}
