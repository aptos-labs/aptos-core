// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Common CLI option structs (key input, encoding, gas, REST, profile, prompt)
//! shared by `aptos` and `aptos-move-cli` commands.

use crate::{
    CliConfig, CliError, CliTypedResult, ConfigSearchMode, Network, ProfileConfig,
    DEFAULT_EXPIRATION_SECS, DEFAULT_PROFILE,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
    x25519, PrivateKey, ValidCryptoMaterialStringExt,
};
use aptos_keygen::KeyGen;
use aptos_rest_client::{AptosBaseUrl, Client};
use aptos_types::transaction::authenticator::AuthenticationKey;
use clap::Parser;
use move_core_types::account_address::AccountAddress;
use serde::Serialize;
use std::{fmt::Display, fs::OpenOptions, path::PathBuf, str::FromStr, time::Duration};

// Custom header value to identify the client
const X_APTOS_CLIENT_VALUE: &str = concat!("aptos-cli/", env!("CARGO_PKG_VERSION"));

// ────────────────────────────────────────────────────────────────────────────
// ReplayProtectionType (from transactions.rs)
// ────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, Parser, PartialEq, clap::ValueEnum)]
pub enum ReplayProtectionType {
    Nonce,
    #[default]
    Seqnum,
}

impl Display for ReplayProtectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            ReplayProtectionType::Nonce => "nonce",
            ReplayProtectionType::Seqnum => "seqnum",
        })
    }
}

// ────────────────────────────────────────────────────────────────────────────
// ProfileOptions
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// RngArgs
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// PromptOptions
// ────────────────────────────────────────────────────────────────────────────

/// An insertable option for use with prompts.
#[derive(Clone, Copy, Debug, Default, Parser, PartialEq, Eq)]
pub struct PromptOptions {
    /// Assume yes for all yes/no prompts
    #[clap(short = 'y', long, group = "prompt_options")]
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

// ────────────────────────────────────────────────────────────────────────────
// EncodingOptions
// ────────────────────────────────────────────────────────────────────────────

/// An insertable option for use with encodings.
#[derive(Debug, Default, Parser, Clone, Copy)]
pub struct EncodingOptions {
    /// Encoding of data as one of [base64, bcs, hex]
    #[clap(long, default_value_t = EncodingType::Hex)]
    pub encoding: EncodingType,
}

// ────────────────────────────────────────────────────────────────────────────
// AuthenticationKeyInputOptions
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// ExtractEd25519PublicKey trait
// ────────────────────────────────────────────────────────────────────────────

pub trait ExtractEd25519PublicKey {
    fn extract_public_key(
        &self,
        encoding: EncodingType,
        profile: &ProfileOptions,
    ) -> CliTypedResult<Ed25519PublicKey>;
}

// ────────────────────────────────────────────────────────────────────────────
// PublicKeyInputOptions
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// ParseEd25519PrivateKey trait
// ────────────────────────────────────────────────────────────────────────────

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
            let key = crate::strip_private_key_prefix(key)?.as_bytes().to_vec();
            Ok(Some(encoding.decode_key("--private-key", key)?))
        } else {
            Ok(None)
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// HardwareWalletOptions
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// PrivateKeyInputOptions
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Parser)]
pub struct PrivateKeyInputOptions {
    /// Signing Ed25519 private key file path
    ///
    /// Encoded with type from `--encoding`
    /// Mutually exclusive with `--private-key`
    #[clap(long, group = "private_key_input", value_parser)]
    pub private_key_file: Option<PathBuf>,
    /// Signing Ed25519 private key
    ///
    /// Encoded with type from `--encoding`
    /// Mutually exclusive with `--private-key-file`
    #[clap(long, group = "private_key_input")]
    pub private_key: Option<String>,
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
            crate::read_from_file(file)
        } else if let Some(ref key) = self.private_key {
            Ok(crate::strip_private_key_prefix(key)?.as_bytes().to_vec())
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

// ────────────────────────────────────────────────────────────────────────────
// account_address_from_public_key / account_address_from_auth_key
// ────────────────────────────────────────────────────────────────────────────

pub fn account_address_from_public_key(public_key: &Ed25519PublicKey) -> AccountAddress {
    let auth_key = AuthenticationKey::ed25519(public_key);
    account_address_from_auth_key(&auth_key)
}

pub fn account_address_from_auth_key(auth_key: &AuthenticationKey) -> AccountAddress {
    AccountAddress::new(*auth_key.account_address())
}

// ────────────────────────────────────────────────────────────────────────────
// SaveFile
// ────────────────────────────────────────────────────────────────────────────

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
        crate::check_if_file_exists(self.output_file.as_path(), self.prompt_options)
    }

    /// Save to the `output_file`
    pub fn save_to_file(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        crate::write_to_file(self.output_file.as_path(), name, bytes)
    }

    /// Save to the `output_file` with restricted permissions (mode 0600)
    pub fn save_to_file_confidential(&self, name: &str, bytes: &[u8]) -> CliTypedResult<()> {
        let mut opts = OpenOptions::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        crate::write_to_file_with_opts(self.output_file.as_path(), name, bytes, &mut opts)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// RestOptions
// ────────────────────────────────────────────────────────────────────────────

/// Options specific to using the Rest endpoint
#[derive(Debug, Parser)]
pub struct RestOptions {
    /// URL to a fullnode on the network
    ///
    /// Defaults to the URL in the `default` profile
    #[clap(long)]
    pub url: Option<reqwest::Url>,

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

// ────────────────────────────────────────────────────────────────────────────
// AccountAddressWrapper
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// load_account_arg
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// MoveManifestAccountWrapper / load_manifest_account_arg
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// FaucetOptions
// ────────────────────────────────────────────────────────────────────────────

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

    pub fn faucet_url(&self, profile_options: &ProfileOptions) -> CliTypedResult<reqwest::Url> {
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
        crate::fund_account(
            rest_client,
            self.faucet_url(profile)?,
            self.faucet_auth_token.as_deref(),
            address,
            num_octas,
        )
        .await
    }
}

// ────────────────────────────────────────────────────────────────────────────
// GasOptions
// ────────────────────────────────────────────────────────────────────────────

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

// ────────────────────────────────────────────────────────────────────────────
// AccountType
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum AccountType {
    Local,
    HardwareWallet,
}

// ────────────────────────────────────────────────────────────────────────────
// TransactionOptions
// ────────────────────────────────────────────────────────────────────────────

/// Common options for interacting with an account for a validator
#[derive(Debug, Default, Parser)]
pub struct TransactionOptions {
    /// Sender account address
    ///
    /// This allows you to override the account address from the derived account address
    /// in the event that the authentication key was rotated or for a resource account
    #[clap(long, value_parser = crate::load_account_arg)]
    pub sender_account: Option<AccountAddress>,

    #[clap(flatten)]
    pub private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub encoding_options: EncodingOptions,
    #[clap(flatten)]
    pub profile_options: ProfileOptions,
    #[clap(flatten)]
    pub rest_options: RestOptions,
    #[clap(flatten)]
    pub gas_options: GasOptions,
    #[clap(flatten)]
    pub prompt_options: PromptOptions,

    /// If this option is set, simulate the transaction locally.
    #[clap(long)]
    pub local: bool,

    /// If this option is set, benchmark the transaction locally.
    #[clap(long)]
    pub benchmark: bool,

    /// If this option is set, simulate the transaction locally using the debugger and generate
    /// flamegraphs that reflect the gas usage.
    #[clap(long)]
    pub profile_gas: bool,

    /// If set, fold the call graph by unique stack traces before generating the gas profile report.
    /// This helps reduce the size of large reports by aggregating identical call paths.
    #[clap(long, requires("profile_gas"))]
    pub fold_unique_stack: bool,

    /// If this option is set, simulate the transaction using a local session.
    #[clap(long)]
    pub session: Option<PathBuf>,

    /// Replay protection mechanism to use when generating the transaction.
    ///
    /// When "nonce" is chosen, the transaction will be an orderless transaction and contains a replay protection nonce.
    ///
    /// When "seqnum" is chosen, the transaction will contain a sequence number that matches with the sender's onchain sequence number.
    #[clap(long, default_value_t = ReplayProtectionType::Seqnum)]
    pub replay_protection_type: ReplayProtectionType,
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
}

// ────────────────────────────────────────────────────────────────────────────
// OptionalPoolAddressArgs / PoolAddressArgs
// ────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
pub struct OptionalPoolAddressArgs {
    /// Address of the Staking pool
    ///
    /// Defaults to the profile's `AccountAddress`
    #[clap(long, value_parser = crate::load_account_arg)]
    pub pool_address: Option<AccountAddress>,
}

#[derive(Parser)]
pub struct PoolAddressArgs {
    /// Address of the Staking pool
    #[clap(long, value_parser = crate::load_account_arg)]
    pub pool_address: AccountAddress,
}

// ────────────────────────────────────────────────────────────────────────────
// MultisigAccount / MultisigAccountWithSequenceNumber
// ────────────────────────────────────────────────────────────────────────────

/// Common options for interactions with a multisig account.
#[derive(Clone, Debug, Parser, Serialize)]
pub struct MultisigAccount {
    /// The address of the multisig account to interact with
    #[clap(long, value_parser = crate::load_account_arg)]
    pub multisig_address: AccountAddress,
}

#[derive(Clone, Debug, Parser, Serialize)]
pub struct MultisigAccountWithSequenceNumber {
    #[clap(flatten)]
    pub multisig_account: MultisigAccount,
    /// Multisig account sequence number to interact with
    #[clap(long)]
    pub sequence_number: u64,
}

// ────────────────────────────────────────────────────────────────────────────
// get_mint_site_url
// ────────────────────────────────────────────────────────────────────────────

/// For minting testnet APT.
pub fn get_mint_site_url(address: Option<AccountAddress>) -> String {
    let params = match address {
        Some(address) => format!("?address={}", address.to_standard_string()),
        None => "".to_string(),
    };
    format!("https://aptos.dev/network/faucet{}", params)
}
