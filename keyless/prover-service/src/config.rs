// Copyright © Aptos Foundation

use crate::metrics;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use rust_rapidsnark::FullProver;
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_PATH: &str = "config.yml";
pub const ZKEY_FILE_PATH : &'static str = "/usr/local/share/aptos-prover-service/prover_key.zkey";
pub const RESOURCES_DIR : &'static str = "/usr/local/share/aptos-prover-service";
pub const ZKEY_FILE_URL : &'static str = "https://github.com/aptos-labs/devnet-groth16-keys/raw/master/main_c.zkey";
pub const ZKEY_SHASUM : &'static str = "29fc68fe5f44bb32cb4fc972069bd5e9f1dcbaf8f72afcac83e1fa57b0307686";
pub const VKEY_FILE_PATH : &'static str = "/usr/local/share/aptos-prover-service/verification_key.json";
pub const VKEY_FILE_URL : &'static str = "https://github.com/aptos-labs/devnet-groth16-keys/raw/master/verification_key.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct ProverServerConfig {
    pub zkey_path: String,
    pub witness_gen_binary_folder_path: String,
    pub test_verification_key_path: String,
    pub oidc_providers: Vec<OidcProvider>,
    pub jwk_refresh_rate_secs: u64,
    pub port: u16,
    pub metrics_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcProvider {
    pub iss: String,
    pub endpoint_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProverServerSecrets {
    pub private_key: Ed25519PrivateKey,
}

pub struct ProverServerState {
    pub full_prover: FullProver,
    pub public_key: Ed25519PublicKey,
    pub private_key: Ed25519PrivateKey,
    pub metrics: metrics::ProverServerMetrics,
}
