// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            account_address_from_public_key, CliError, CliTypedResult, EncodingOptions, KeyType,
            PrivateKeyInputOptions, RngArgs, SaveFile,
        },
        utils::{
            append_file_extension, check_if_file_exists, generate_vanity_account_ed25519,
            write_to_file,
        },
    },
    CliCommand, CliResult,
};
use velor_config::config::{Peer, PeerRole};
use velor_crypto::{
    bls12381, ed25519, ed25519::Ed25519PrivateKey, encoding_type::EncodingType, x25519, PrivateKey,
    ValidCryptoMaterial,
};
use velor_genesis::config::HostAndPort;
use velor_types::account_address::{
    create_multisig_account_address, from_identity_public_key, AccountAddress,
};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub const PUBLIC_KEY_EXTENSION: &str = "pub";

/// Tool for generating, inspecting, and interacting with keys
///
/// This tool allows users to generate and extract related information
/// with all key types used on the Velor blockchain.
#[derive(Debug, Subcommand)]
pub enum KeyTool {
    Generate(GenerateKey),
    ExtractPublicKey(ExtractPublicKey),
    ExtractPeer(ExtractPeer),
}

impl KeyTool {
    pub async fn execute(self) -> CliResult {
        match self {
            KeyTool::Generate(tool) => tool.execute_serialized().await,
            KeyTool::ExtractPeer(tool) => tool.execute_serialized().await,
            KeyTool::ExtractPublicKey(tool) => tool.execute_serialized().await,
        }
    }
}

/// Extract full peer information for an upstream peer
///
/// This command builds a YAML blob that can be copied into a user's network configuration.
/// A host is required to build the network address used for the connection, and the
/// network key is required to identify the peer.
///
/// A `private-network-key` or `public-network-key` can be given encoded on the command line, or
/// a `private-network-key-file` or a `public-network-key-file` can be given to read from.
/// The `output-file` will be a YAML serialized peer information for use in network config.
#[derive(Debug, Parser)]
pub struct ExtractPeer {
    /// Host and port of the full node
    ///
    /// e.g. 127.0.0.1:6180 or my-awesome-dns.com:6180
    #[clap(long)]
    pub(crate) host: HostAndPort,

    #[clap(flatten)]
    pub(crate) network_key_input_options: NetworkKeyInputOptions,
    #[clap(flatten)]
    pub(crate) output_file_options: SaveFile,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
}

#[async_trait]
impl CliCommand<HashMap<AccountAddress, Peer>> for ExtractPeer {
    fn command_name(&self) -> &'static str {
        "ExtractPeer"
    }

    async fn execute(self) -> CliTypedResult<HashMap<AccountAddress, Peer>> {
        // Load key based on public or private
        let public_key = self
            .network_key_input_options
            .extract_public_network_key(self.encoding_options.encoding)?;

        // Check output file exists
        self.output_file_options.check_file()?;

        // Build peer info
        let peer_id = from_identity_public_key(public_key);
        let mut public_keys = HashSet::new();
        public_keys.insert(public_key);

        let address = self.host.as_network_address(public_key).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to build network address: {}", err))
        })?;

        let peer = Peer::new(vec![address], public_keys, PeerRole::Upstream);

        let mut map = HashMap::new();
        map.insert(peer_id, peer);

        // Save to file
        let yaml = serde_yaml::to_string(&map)
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
        self.output_file_options
            .save_to_file("Extracted peer", yaml.as_bytes())?;
        Ok(map)
    }
}

#[derive(Debug, Default, Parser)]
pub struct NetworkKeyInputOptions {
    /// x25519 Private key input file name
    #[clap(long, group = "network_key_input", value_parser)]
    private_network_key_file: Option<PathBuf>,

    /// x25519 Private key encoded in a type as shown in `encoding`
    #[clap(long, group = "network_key_input")]
    private_network_key: Option<String>,

    /// x25519 Public key input file name
    #[clap(long, group = "network_key_input", value_parser)]
    public_network_key_file: Option<PathBuf>,

    /// x25519 Public key encoded in a type as shown in `encoding`
    #[clap(long, group = "network_key_input")]
    public_network_key: Option<String>,
}

impl NetworkKeyInputOptions {
    pub fn from_private_key_file(file: PathBuf) -> Self {
        Self {
            private_network_key_file: Some(file),
            private_network_key: None,
            public_network_key_file: None,
            public_network_key: None,
        }
    }

    pub fn extract_public_network_key(
        self,
        encoding: EncodingType,
    ) -> CliTypedResult<x25519::PublicKey> {
        // The grouping above prevents there from being more than one, but just in case
        match (self.public_network_key, self.public_network_key_file, self.private_network_key, self.private_network_key_file) {
            (Some(public_network_key), None, None, None) => Ok(encoding.decode_key("--public-network-key", public_network_key.as_bytes().to_vec())?),
            (None, Some(public_network_key_file), None, None) => Ok(encoding.load_key("--public-network-key-file", public_network_key_file.as_path())?),
            (None, None, Some(private_network_key), None) => {
                let private_network_key: x25519::PrivateKey = encoding.decode_key("--private-network-key", private_network_key.as_bytes().to_vec())?;
                Ok(private_network_key.public_key())
            }
            (None, None, None, Some(private_network_key_file)) => {
                let private_network_key: x25519::PrivateKey = encoding.load_key("--private-network-key-file", private_network_key_file.as_path())?;
                Ok(private_network_key.public_key())
            }
            _ => Err(CliError::CommandArgumentError("Must provide exactly one of [--public-network-key, --public-network-key-file, --private-network-key, --private-network-key-file]".to_string()))
        }
    }
}

/// Generates a `x25519`, `ed25519` or `bls12381` key.
///
/// This can be used for generating an identity.  Two files will be created
/// `output_file` and `output_file.pub`.  `output_file` will contain the private
/// key encoded with the `encoding` and `output_file.pub` will contain the public
/// key encoded with the `encoding`.
#[derive(Debug, Parser)]
pub struct GenerateKey {
    /// Key type to generate. Must be one of [x25519, ed25519, bls12381]
    #[clap(long, default_value_t = KeyType::Ed25519)]
    pub(crate) key_type: KeyType,
    /// Vanity prefix that resultant account address should start with, e.g. 0xaceface or d00d. Each
    /// additional character multiplies by a factor of 16 the computational difficulty associated
    /// with generating an address, so try out shorter prefixes first and be prepared to wait for
    /// longer ones
    #[clap(long)]
    pub vanity_prefix: Option<String>,
    /// Use this flag when vanity prefix is for a multisig account. This mines a private key for
    /// a single signer account that can, as its first transaction, create a multisig account with
    /// the given vanity prefix
    #[clap(long)]
    pub vanity_multisig: bool,
    #[clap(flatten)]
    pub rng_args: RngArgs,
    #[clap(flatten)]
    pub(crate) save_params: SaveKey,
}

#[async_trait]
impl CliCommand<HashMap<&'static str, PathBuf>> for GenerateKey {
    fn command_name(&self) -> &'static str {
        "GenerateKey"
    }

    async fn execute(self) -> CliTypedResult<HashMap<&'static str, PathBuf>> {
        if self.vanity_prefix.is_some() && !matches!(self.key_type, KeyType::Ed25519) {
            return Err(CliError::CommandArgumentError(format!(
                "Vanity prefixes are only accepted for {} keys",
                KeyType::Ed25519
            )));
        }
        if self.vanity_multisig && self.vanity_prefix.is_none() {
            return Err(CliError::CommandArgumentError(
                "No vanity prefix provided".to_string(),
            ));
        }
        self.save_params.check_key_file()?;
        let mut keygen = self.rng_args.key_generator()?;
        match self.key_type {
            KeyType::X25519 => {
                let private_key = keygen.generate_x25519_private_key().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Failed to convert ed25519 to x25519 {:?}",
                        err
                    ))
                })?;
                self.save_params.save_key(&private_key, "x25519")
            },
            KeyType::Ed25519 => {
                // If no vanity prefix specified, generate a standard Ed25519 private key.
                let private_key = if self.vanity_prefix.is_none() {
                    keygen.generate_ed25519_private_key()
                } else {
                    // If a vanity prefix is specified, generate vanity Ed25519 account from it.
                    generate_vanity_account_ed25519(
                        self.vanity_prefix.clone().unwrap().as_str(),
                        self.vanity_multisig,
                    )?
                };
                // Store CLI result from key save operation, to append vanity address(es) if needed.
                let mut result_map = self.save_params.save_key(&private_key, "ed25519").unwrap();
                if self.vanity_prefix.is_some() {
                    let account_address = account_address_from_public_key(
                        &ed25519::Ed25519PublicKey::from(&private_key),
                    );
                    // Store account address in a PathBuf so it can be displayed in CLI result.
                    result_map.insert(
                        "Account Address:",
                        PathBuf::from(account_address.to_hex_literal()),
                    );
                    if self.vanity_multisig {
                        let multisig_account_address =
                            create_multisig_account_address(account_address, 0);
                        result_map.insert(
                            "Multisig Account Address:",
                            PathBuf::from(multisig_account_address.to_hex_literal()),
                        );
                    }
                }
                return Ok(result_map);
            },
            KeyType::Bls12381 => {
                let private_key = keygen.generate_bls12381_private_key();
                self.save_params.save_bls_key(&private_key, "bls12381")
            },
        }
    }
}

impl GenerateKey {
    /// A test friendly typed key generation for x25519 keys.
    pub async fn generate_x25519(
        encoding: EncodingType,
        key_file: &Path,
    ) -> CliTypedResult<(x25519::PrivateKey, x25519::PublicKey)> {
        let args = format!(
            "generate --key-type {key_type:?} --output-file {key_file} --encoding {encoding:?} --assume-yes",
            key_type = KeyType::X25519,
            key_file = key_file.display(),
            encoding = encoding,
        );
        let command = GenerateKey::parse_from(args.split_whitespace());
        command.execute().await?;
        Ok((
            encoding.load_key("private_key", key_file)?,
            encoding.load_key(
                "public_key",
                &append_file_extension(key_file, PUBLIC_KEY_EXTENSION)?,
            )?,
        ))
    }

    /// A test friendly typed key generation for e25519 keys.
    pub async fn generate_ed25519(
        encoding: EncodingType,
        key_file: &Path,
    ) -> CliTypedResult<(ed25519::Ed25519PrivateKey, ed25519::Ed25519PublicKey)> {
        let args = format!(
            "generate --key-type {key_type:?} --output-file {key_file} --encoding {encoding:?} --assume-yes",
            key_type = KeyType::Ed25519,
            key_file = key_file.display(),
            encoding = encoding,
        );
        let command = GenerateKey::parse_from(args.split_whitespace());
        command.execute().await?;
        Ok((
            encoding.load_key("private_key", key_file)?,
            encoding.load_key(
                "public_key",
                &append_file_extension(key_file, PUBLIC_KEY_EXTENSION)?,
            )?,
        ))
    }
}

/// Extracts the public key and any appropriate proof of possession from the PrivateKey
///
/// You can simply run this by using the same kinds of inputs as Generate
///
/// ```bash
/// velor key extract-public-key --private-key-file ./path-to-key --output-file ./path-to-output --key-type bls12381
/// ```
#[derive(Debug, Parser)]
pub struct ExtractPublicKey {
    /// Key type to generate. Must be one of [x25519, ed25519, bls12381]
    #[clap(long, default_value_t = KeyType::Ed25519)]
    pub(crate) key_type: KeyType,
    #[clap(flatten)]
    pub(crate) private_key_params: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub rng_args: RngArgs,
    #[clap(flatten)]
    pub(crate) save_params: SaveKey,
}

#[async_trait]
impl CliCommand<HashMap<&'static str, PathBuf>> for ExtractPublicKey {
    fn command_name(&self) -> &'static str {
        "ExtractPublicKey"
    }

    async fn execute(self) -> CliTypedResult<HashMap<&'static str, PathBuf>> {
        let private_key_bytes = self
            .private_key_params
            .extract_private_key_input_from_cli_args()?;
        let files = match self.key_type {
            KeyType::Ed25519 => {
                let key = self
                    .save_params
                    .encoding_options
                    .encoding
                    .decode_key::<Ed25519PrivateKey>("ed25519 private key", private_key_bytes)?;
                vec![self.save_params.save_material(
                    &key.public_key(),
                    "ed25519 public key",
                    PUBLIC_KEY_EXTENSION,
                )?]
            },
            KeyType::X25519 => {
                let key = self
                    .save_params
                    .encoding_options
                    .encoding
                    .decode_key::<x25519::PrivateKey>("ed25519 private key", private_key_bytes)?;
                vec![self.save_params.save_material(
                    &key.public_key(),
                    "x25519 public key",
                    PUBLIC_KEY_EXTENSION,
                )?]
            },
            KeyType::Bls12381 => {
                let key = self
                    .save_params
                    .encoding_options
                    .encoding
                    .decode_key::<bls12381::PrivateKey>(
                        "bls12381 private key",
                        private_key_bytes,
                    )?;
                vec![
                    self.save_params.clone().save_material(
                        &key.public_key(),
                        "bls12381 public key",
                        PUBLIC_KEY_EXTENSION,
                    )?,
                    self.save_params.save_material(
                        &bls12381::ProofOfPossession::create(&key),
                        "bls12381 proof of possession",
                        "pop",
                    )?,
                ]
            },
        };
        Ok(HashMap::from_iter(files))
    }
}

#[derive(Debug, Parser, Clone)]
pub struct SaveKey {
    #[clap(flatten)]
    pub(crate) file_options: SaveFile,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
}

impl SaveKey {
    /// Public key file name
    fn public_key_file(&self) -> CliTypedResult<PathBuf> {
        append_file_extension(
            self.file_options.output_file.as_path(),
            PUBLIC_KEY_EXTENSION,
        )
    }

    /// Public key file name
    fn proof_of_possession_file(&self) -> CliTypedResult<PathBuf> {
        append_file_extension(self.file_options.output_file.as_path(), "pop")
    }

    /// Check if the key file exists already
    pub fn check_key_file(&self) -> CliTypedResult<()> {
        // Check if file already exists
        self.file_options.check_file()?;
        check_if_file_exists(&self.public_key_file()?, self.file_options.prompt_options)
    }

    /// Saves a key to a file encoded in a string
    pub fn save_key<Key: PrivateKey + ValidCryptoMaterial>(
        self,
        key: &Key,
        key_name: &'static str,
    ) -> CliTypedResult<HashMap<&'static str, PathBuf>> {
        let encoded_private_key = self.encoding_options.encoding.encode_key(key_name, key)?;
        let encoded_public_key = self
            .encoding_options
            .encoding
            .encode_key(key_name, &key.public_key())?;

        // Write private and public keys to files
        let public_key_file = self.public_key_file()?;
        self.file_options
            .save_to_file_confidential(key_name, &encoded_private_key)?;
        write_to_file(&public_key_file, key_name, &encoded_public_key)?;

        let mut map = HashMap::new();
        map.insert("PrivateKey Path", self.file_options.output_file);
        map.insert("PublicKey Path", public_key_file);
        Ok(map)
    }

    /// Saves material to an enocded file
    pub fn save_material<Key: ValidCryptoMaterial>(
        self,
        material: &Key,
        name: &'static str,
        extension: &'static str,
    ) -> CliTypedResult<(&'static str, PathBuf)> {
        let encoded_material = self.encoding_options.encoding.encode_key(name, material)?;
        let file = append_file_extension(self.file_options.output_file.as_path(), extension)?;
        write_to_file(&file, name, &encoded_material)?;
        Ok((name, file))
    }

    /// Saves a key to a file encoded in a string
    pub fn save_bls_key(
        self,
        key: &bls12381::PrivateKey,
        key_name: &'static str,
    ) -> CliTypedResult<HashMap<&'static str, PathBuf>> {
        let encoded_private_key = self.encoding_options.encoding.encode_key(key_name, key)?;
        let encoded_public_key = self
            .encoding_options
            .encoding
            .encode_key(key_name, &key.public_key())?;
        let encoded_proof_of_posession = self
            .encoding_options
            .encoding
            .encode_key(key_name, &bls12381::ProofOfPossession::create(key))?;

        // Write private and public keys to files
        let public_key_file = self.public_key_file()?;
        let proof_of_possession_file = self.proof_of_possession_file()?;
        self.file_options
            .save_to_file_confidential(key_name, &encoded_private_key)?;
        write_to_file(&public_key_file, key_name, &encoded_public_key)?;
        write_to_file(
            &proof_of_possession_file,
            key_name,
            &encoded_proof_of_posession,
        )?;

        let mut map = HashMap::new();
        map.insert("PrivateKey Path", self.file_options.output_file);
        map.insert("PublicKey Path", public_key_file);
        map.insert("Proof of possession Path", proof_of_possession_file);
        Ok(map)
    }
}
