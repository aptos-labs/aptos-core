// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::error::PepperServiceError;
use aptos_crypto::blstrs::scalar_from_uniform_be_bytes;
use aptos_keyless_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF},
    PepperV0VufPubKey,
};
use aptos_logger::{info, warn};
use blstrs::{G2Projective, Scalar};
use sha3::Digest;

/// A simple struct that holds a VUF public and private keypair,
/// including the VUF public key in JSON format.
pub struct VUFKeypair {
    vuf_private_key: Scalar,
    vuf_public_key: G2Projective,
    vuf_public_key_json: String,
}

impl VUFKeypair {
    pub fn new(
        vuf_private_key: Scalar,
        vuf_public_key: G2Projective,
        vuf_public_key_json: String,
    ) -> Self {
        Self {
            vuf_private_key,
            vuf_public_key,
            vuf_public_key_json,
        }
    }

    /// Returns a reference to the VUF private key
    pub fn vuf_private_key(&self) -> &Scalar {
        &self.vuf_private_key
    }

    /// Returns a reference to the VUF public key
    pub fn vuf_public_key(&self) -> &G2Projective {
        &self.vuf_public_key
    }

    /// Returns a reference to the VUF public key in JSON format
    pub fn vuf_public_key_json(&self) -> &String {
        &self.vuf_public_key_json
    }
}

/// Derive the VUF private key from the given hex-encoded seed
fn derive_vuf_private_key_from_seed(
    vuf_private_key_seed_hex: String,
) -> Result<Scalar, PepperServiceError> {
    // Decode the hex-encoded seed
    let vuf_private_key_seed = hex::decode(vuf_private_key_seed_hex).map_err(|error| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to decode VUF private key seed hex! Error: {}",
            error
        ))
    })?;

    // Ensure the seed has sufficient entropy (at least 32 bytes)
    let vuf_private_key_seed_length = vuf_private_key_seed.len();
    if vuf_private_key_seed_length < 32 {
        return Err(PepperServiceError::UnexpectedError(format!(
            "VUF private key seed must be at least 32 bytes long! Found length: {}",
            vuf_private_key_seed_length
        )));
    }

    // Hash the seed to derive the private key
    let mut sha3_hasher = sha3::Sha3_512::new();
    sha3_hasher.update(vuf_private_key_seed);
    let vuf_private_key = scalar_from_uniform_be_bytes(sha3_hasher.finalize().as_slice());

    Ok(vuf_private_key)
}

/// Deserialize the VUF private key directly from the given hex-encoded string
fn deserialize_vuf_private_key(vuf_private_key_hex: String) -> Result<Scalar, PepperServiceError> {
    // Decode the hex-encoded private key
    let vuf_private_key_bytes = hex::decode(vuf_private_key_hex).map_err(|e| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to decode VUF private key hex! Error: {}",
            e
        ))
    })?;

    // Deserialize the private key bytes
    let vuf_private_key_opt =
        Scalar::from_bytes_le(<&[u8; 32]>::try_from(vuf_private_key_bytes.as_slice()).unwrap());

    if vuf_private_key_opt.is_none().unwrap_u8() == 1 {
        return Err(PepperServiceError::UnexpectedError(
            "Failed to deserialize VUF private key!".to_string(),
        ));
    }

    Ok(vuf_private_key_opt.unwrap())
}

/// Fetches the VUF private key for the pepper service. Note: this function
/// will panic if no valid private key can be derived or deserialized.
fn get_pepper_service_vuf_private_key(
    vuf_private_key_hex: Option<String>,
    vuf_private_key_seed_hex: Option<String>,
) -> Scalar {
    info!("Loading the VUF private key for the pepper service...");

    // Attempt to derive the private key from the seed
    if let Some(vuf_private_key_seed_hex) = vuf_private_key_seed_hex {
        match derive_vuf_private_key_from_seed(vuf_private_key_seed_hex) {
            Ok(private_key) => {
                info!("Successfully derived the VUF private key from the seed!");
                return private_key;
            },
            Err(error) => {
                warn!(
                    "Failed to derive the VUF private key from a seed! Error: {}",
                    error
                );
            },
        }
    }

    // Seed derivation failed, fall back to direct deserialization
    warn!("Falling back to direct VUF private key deserialization!");

    // Attempt to deserialize the private key directly
    if let Some(vuf_private_key_hex) = vuf_private_key_hex {
        match deserialize_vuf_private_key(vuf_private_key_hex) {
            Ok(private_key) => {
                info!("Successfully deserialized the VUF private key directly!");
                return private_key;
            },
            Err(error) => {
                warn!(
                    "Failed to derive VUF private key from direct deserialization! Error: {}",
                    error
                );
            },
        }
    }

    // Panic, as there is no valid private key
    panic!("No valid VUF private key could be derived or deserialized!");
}

/// Returns the VUF public key of the pepper service (both the G2Projective and JSON)
pub fn get_pepper_service_vuf_public_key_and_json(
    vuf_private_key: &Scalar,
) -> (G2Projective, String) {
    info!("Deriving the VUF public key for the pepper service...");

    // Calculate the corresponding public key
    let vuf_public_key = match Bls12381G1Bls::pk_from_sk(vuf_private_key) {
        Ok(public_key) => public_key,
        Err(error) => panic!(
            "Failed to derive the VUF public key from the private key! Error: {}",
            error
        ),
    };

    // Create the public key object
    let public_key_buf = vuf_public_key.to_compressed().to_vec();
    let pepper_vuf_public_key = PepperV0VufPubKey::new(public_key_buf);
    info!(
        "Successfully derived the VUF public key for the pepper service: {:?}",
        pepper_vuf_public_key
    );

    // Transform the public key to a pretty JSON string
    let vuf_public_key_json = serde_json::to_string_pretty(&pepper_vuf_public_key)
        .expect("Fail to format the VUF public key using pretty JSON!");

    (vuf_public_key, vuf_public_key_json)
}

/// Returns the VUF keypair for the pepper service
pub fn get_pepper_service_vuf_keypair(
    vuf_private_key_hex: Option<String>,
    vuf_private_key_seed_hex: Option<String>,
) -> VUFKeypair {
    // Fetch the VUF private key
    let vuf_private_key =
        get_pepper_service_vuf_private_key(vuf_private_key_hex, vuf_private_key_seed_hex);

    // Get the VUF public key and its JSON representation
    let (vuf_public_key, vuf_public_key_json) =
        get_pepper_service_vuf_public_key_and_json(&vuf_private_key);

    // Create a new VUFKeypair
    VUFKeypair::new(vuf_private_key, vuf_public_key, vuf_public_key_json)
}

#[cfg(test)]
mod tests {
    use crate::vuf_keypair;

    // Useful test constants
    const TEST_VUF_KEY_SEED: &str =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    const TEST_VUF_KEY_HEX: &str =
        "0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9";

    #[test]
    #[should_panic(expected = "No valid VUF private key could be derived or deserialized!")]
    fn test_get_pepper_service_vuf_keypair() {
        // Get the pepper service VUF keypair given both seed and hex
        let vuf_keypair = vuf_keypair::get_pepper_service_vuf_keypair(
            Some(TEST_VUF_KEY_HEX.into()),
            Some(TEST_VUF_KEY_SEED.into()),
        );

        // Verify the private key is derived from the seed
        let derived_private_key =
            vuf_keypair::derive_vuf_private_key_from_seed(TEST_VUF_KEY_SEED.into()).unwrap();
        assert_eq!(vuf_keypair.vuf_private_key(), &derived_private_key);

        // Verify the public keys are correctly derived
        let (expected_public_key, expected_public_key_json) =
            vuf_keypair::get_pepper_service_vuf_public_key_and_json(vuf_keypair.vuf_private_key());
        assert_eq!(vuf_keypair.vuf_public_key(), &expected_public_key);
        assert_eq!(vuf_keypair.vuf_public_key_json(), &expected_public_key_json);

        // Get the pepper service VUF keypair again (with only the hex)
        let vuf_keypair =
            vuf_keypair::get_pepper_service_vuf_keypair(Some(TEST_VUF_KEY_HEX.into()), None);

        // Verify the private key is derived from the fallback hex
        let deserialized_private_key =
            vuf_keypair::deserialize_vuf_private_key(TEST_VUF_KEY_HEX.to_string()).unwrap();
        assert_eq!(vuf_keypair.vuf_private_key(), &deserialized_private_key);

        // Verify the public keys are correctly derived
        let (expected_public_key, expected_public_key_json) =
            vuf_keypair::get_pepper_service_vuf_public_key_and_json(vuf_keypair.vuf_private_key());
        assert_eq!(vuf_keypair.vuf_public_key(), &expected_public_key);
        assert_eq!(vuf_keypair.vuf_public_key_json(), &expected_public_key_json);

        // Get the pepper service VUF keypair again, with neither seed nor hex (this should panic)
        let _ = vuf_keypair::get_pepper_service_vuf_keypair(None, None);
    }
}
