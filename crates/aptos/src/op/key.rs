// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            CliError, CliTypedResult, EncodingOptions, EncodingType, KeyType, RngArgs, SaveFile,
        },
        utils::{append_file_extension, check_if_file_exists, write_to_file},
    },
    CliCommand, CliResult,
};
use aptos_config::config::{Peer, PeerRole};
use aptos_crypto::{ed25519, x25519, PrivateKey, ValidCryptoMaterial};
use aptos_genesis::config::HostAndPort;
use aptos_types::account_address::{from_identity_public_key, AccountAddress};
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
/// with all key types used on the Aptos blockchain.
#[derive(Debug, Subcommand)]
pub enum KeyTool {
    Generate(GenerateKey),
    ExtractPeer(ExtractPeer),
}

impl KeyTool {
    pub async fn execute(self) -> CliResult {
        match self {
            KeyTool::Generate(tool) => tool.execute_serialized().await,
            KeyTool::ExtractPeer(tool) => tool.execute_serialized().await,
        }
    }
}

/// CLI tool for extracting full peer information for an upstream peer
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
    #[clap(long, group = "network_key_input", parse(from_os_str))]
    private_network_key_file: Option<PathBuf>,

    /// x25519 Private key encoded in a type as shown in `encoding`
    #[clap(long, group = "network_key_input")]
    private_network_key: Option<String>,

    /// x25519 Public key input file name
    #[clap(long, group = "network_key_input", parse(from_os_str))]
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
        match (self.public_network_key,  self.public_network_key_file, self.private_network_key, self.private_network_key_file){
            (Some(public_network_key), None, None, None) => encoding.decode_key("--public-network-key", public_network_key.as_bytes().to_vec()),
            (None, Some(public_network_key_file),None,  None) => encoding.load_key("--public-network-key-file", public_network_key_file.as_path()),
            (None, None, Some(private_network_key),  None) => {
                let private_network_key: x25519::PrivateKey = encoding.decode_key("--private-network-key", private_network_key.as_bytes().to_vec())?;
                Ok(private_network_key.public_key())
            },
            (None, None, None, Some(private_network_key_file)) => {
                let private_network_key: x25519::PrivateKey = encoding.load_key("--private-network-key-file", private_network_key_file.as_path())?;
                Ok(private_network_key.public_key())
            },
            _ => Err(CliError::CommandArgumentError("Must provide exactly one of [--public-network-key, --public-network-key-file, --private-network-key, --private-network-key-file]".to_string()))
        }
    }
}

/// Generates a `x25519` or `ed25519` key.
///
/// This can be used for generating an identity.  Two files will be created
/// `output_file` and `output_file.pub`.  `output_file` will contain the private
/// key encoded with the `encoding` and `output_file.pub` will contain the public
/// key encoded with the `encoding`.
#[derive(Debug, Parser)]
pub struct GenerateKey {
    /// Key type to generate. Must be one of [x25519, ed25519]
    #[clap(long, default_value_t = KeyType::Ed25519)]
    pub(crate) key_type: KeyType,

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
            }
            KeyType::Ed25519 => {
                let private_key = keygen.generate_ed25519_private_key();
                self.save_params.save_key(&private_key, "ed25519")
            }
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

#[derive(Debug, Parser)]
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
}
