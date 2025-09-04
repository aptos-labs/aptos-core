// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::PepperServiceError, utils};
use velor_keyless_pepper_common::{
    vuf::{bls12381_g1_bls::Bls12381G1Bls, VUF},
    PepperV0VufPubKey,
};
use velor_logger::{info, warn};
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use sha3::Digest;

// VUF related environment variables
pub const ENV_VUF_KEY_HEX: &str = "VUF_KEY_HEX";
pub const ENV_VUF_KEY_SEED_HEX: &str = "VUF_KEY_SEED_HEX";

/// Derive the VUF private key from the given hex-encoded seed
fn derive_vuf_private_key_from_seed(
    vuf_private_key_seed_hex: String,
) -> Result<ark_bls12_381::Fr, PepperServiceError> {
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
    let vuf_private_key =
        ark_bls12_381::Fr::from_be_bytes_mod_order(sha3_hasher.finalize().as_slice());

    Ok(vuf_private_key)
}

/// Deserialize the VUF private key directly from the given hex-encoded string
fn deserialize_vuf_private_key(
    vuf_private_key_hex: String,
) -> Result<ark_bls12_381::Fr, PepperServiceError> {
    // Decode the hex-encoded private key
    let mut vuf_private_key_bytes = hex::decode(vuf_private_key_hex).map_err(|e| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to decode VUF private key hex! Error: {}",
            e
        ))
    })?;

    // Reverse the bytes (TODO: why do we need to do this?)
    vuf_private_key_bytes.reverse();

    // Deserialize the private key bytes
    let vuf_private_key = ark_bls12_381::Fr::deserialize_compressed(
        vuf_private_key_bytes.as_slice(),
    )
    .map_err(|error| {
        PepperServiceError::UnexpectedError(format!(
            "Failed to deserialize VUF private key! Error: {}",
            error
        ))
    })?;

    Ok(vuf_private_key)
}

/// Fetches the VUF private key for the pepper service. Note: this function
/// will panic if the private key cannot be found (e.g., from the environment variables).
fn get_pepper_service_vuf_private_key() -> ark_bls12_381::Fr {
    info!("Loading the VUF private key for the pepper service...");

    // Load the VUF private key seed from the environment variable
    let vuf_private_key_seed_hex = match utils::read_environment_variable(ENV_VUF_KEY_SEED_HEX) {
        Ok(vuf_private_key_seed_hex) => Some(vuf_private_key_seed_hex),
        Err(error) => {
            warn!(
                "Failed to read the VUF private key seed from the environment variable ({})! Error: {}",
                ENV_VUF_KEY_SEED_HEX, error
            );
            None
        },
    };

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

    // Load the VUF private key hex from the environment variable
    let vuf_private_key_hex = match utils::read_environment_variable(ENV_VUF_KEY_HEX) {
        Ok(vuf_private_key_hex) => Some(vuf_private_key_hex),
        Err(error) => {
            warn!(
                "Failed to read the VUF private key from the environment variable ({})! Error: {}",
                ENV_VUF_KEY_HEX, error
            );
            None
        },
    };

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

/// Returns the VUF public key of the pepper service (in JSON format)
fn get_pepper_service_vuf_public_key(vuf_private_key: &ark_bls12_381::Fr) -> String {
    info!("Deriving the VUF public key for the pepper service...");

    // Fetch the corresponding public key
    let vuf_public_key = match Bls12381G1Bls::pk_from_sk(vuf_private_key) {
        Ok(public_key) => public_key,
        Err(error) => panic!(
            "Failed to derive the VUF public key from the private key! Error: {}",
            error
        ),
    };

    // Create the public key object
    let mut public_key_buf = vec![];
    vuf_public_key
        .into_affine()
        .serialize_compressed(&mut public_key_buf)
        .expect("Failed to serialize the public key!");
    let pepper_vuf_public_key = PepperV0VufPubKey::new(public_key_buf);
    info!(
        "Successfully derived the VUF public key for the pepper service: {:?}",
        pepper_vuf_public_key
    );

    // Transform the public key to a pretty JSON string
    serde_json::to_string_pretty(&pepper_vuf_public_key)
        .expect("Fail to format the VUF public key using pretty JSON!")
}

/// Returns the VUF public and private keypair for the pepper service.
/// Note: the public key is serialized and returned in JSON format.
pub fn get_pepper_service_vuf_keypair() -> (String, ark_bls12_381::Fr) {
    // Fetch the VUF private key, and derive the public key
    let vuf_private_key = get_pepper_service_vuf_private_key();
    let vuf_public_key = get_pepper_service_vuf_public_key(&vuf_private_key);

    // Return the keypair
    (vuf_public_key, vuf_private_key)
}

#[cfg(test)]
mod tests {
    use crate::{
        vuf_pub_key,
        vuf_pub_key::{ENV_VUF_KEY_HEX, ENV_VUF_KEY_SEED_HEX},
    };

    // Useful test constants
    const TEST_VUF_KEY_SEED: &str =
        "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    const TEST_VUF_KEY_HEX: &str =
        "0a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f9";

    #[test]
    #[should_panic(expected = "No valid VUF private key could be derived or deserialized!")]
    fn test_get_pepper_service_vuf_keypair() {
        // Set the private key seed environment variable
        std::env::set_var(ENV_VUF_KEY_SEED_HEX, TEST_VUF_KEY_SEED);

        // Set the fallback private key environment variable
        std::env::set_var(ENV_VUF_KEY_HEX, TEST_VUF_KEY_HEX);

        // Get the pepper service VUF keypair
        let (public_key, private_key) = vuf_pub_key::get_pepper_service_vuf_keypair();

        // Verify the private key is derived from the seed
        let derived_private_key =
            vuf_pub_key::derive_vuf_private_key_from_seed(TEST_VUF_KEY_SEED.to_string()).unwrap();
        assert_eq!(private_key, derived_private_key);

        // Verify the public key is correctly derived from the private key
        let expected_public_key = vuf_pub_key::get_pepper_service_vuf_public_key(&private_key);
        assert_eq!(public_key, expected_public_key);

        // Remove the seed environment variable to force the fallback
        std::env::remove_var(ENV_VUF_KEY_SEED_HEX);

        // Get the pepper service VUF keypair again
        let (public_key, private_key) = vuf_pub_key::get_pepper_service_vuf_keypair();

        // Verify the private key is derived from the fallback hex
        let deserialized_private_key =
            vuf_pub_key::deserialize_vuf_private_key(TEST_VUF_KEY_HEX.to_string()).unwrap();
        assert_eq!(private_key, deserialized_private_key);

        // Verify the public key is correctly derived from the private key
        let expected_public_key = vuf_pub_key::get_pepper_service_vuf_public_key(&private_key);
        assert_eq!(public_key, expected_public_key);

        // Remove all environment variables
        std::env::remove_var(ENV_VUF_KEY_HEX);

        // Get the pepper service VUF keypair again, this should panic
        let _ = vuf_pub_key::get_pepper_service_vuf_keypair();
    }
}
