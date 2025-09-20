// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use anyhow::{anyhow, Result};
use ark_bn254::Bn254;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use serde::{Deserialize, Serialize};

/// This struct is a representation of a Groth16VerificationKey resource as found on-chain.
/// See, for example:
/// https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
///
/// Example JSON:
/// {
///   "type": "0x1::keyless_account::Groth16VerificationKey",
///   "data": {
///     "alpha_g1": "0xe2f26dbea299f5223b646cb1fb33eadb059d9407559d7441dfd902e3a79a4d2d",
///     "beta_g2": "0xabb73dc17fbc13021e2471e0c08bd67d8401f52b73d6d07483794cad4778180e0c06f33bbc4c79a9cadef253a68084d382f17788f885c9afd176f7cb2f036789",
///     "delta_g2": "0xb106619932d0ef372c46909a2492e246d5de739aa140e27f2c71c0470662f125219049cfe15e4d140d7e4bb911284aad1cad19880efb86f2d9dd4b1bb344ef8f",
///     "gamma_abc_g1": [
///       "0x6123b6fea40de2a7e3595f9c35210da8a45a7e8c2f7da9eb4548e9210cfea81a",
///       "0x32a9b8347c512483812ee922dc75952842f8f3083edb6fe8d5c3c07e1340b683"
///     ],
///     "gamma_g2": "0xedf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19"
///   }
/// }
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnChainGroth16VerificationKey {
    pub r#type: String, // Note: "type" is a reserved keyword, so we use raw identifier syntax
    pub data: Groth16VerificationKeyData,
}

impl OnChainGroth16VerificationKey {
    #[cfg(test)]
    /// Creates a new OnChainGroth16VerificationKey for testing.
    /// Note: internally, the fields will contain empty/default values.
    pub fn new_for_testing() -> Self {
        OnChainGroth16VerificationKey {
            r#type: "0x1::keyless_account::Groth16VerificationKey".to_string(),
            data: Groth16VerificationKeyData::default(),
        }
    }

    /// Converts the on-chain verification key to a prepared verifying key
    pub fn to_ark_prepared_verifying_key(&self) -> Result<PreparedVerifyingKey<Bn254>> {
        // Collect and parse the gamma_abc_g1 elements
        let verification_key_data = &self.data;
        let mut gamma_abc_g1 = Vec::with_capacity(verification_key_data.gamma_abc_g1.len());
        for (index, onchain_ele) in verification_key_data.gamma_abc_g1.iter().enumerate() {
            let ark_ele = parse_type_from_hex(onchain_ele).map_err(|error| {
                anyhow!(
                    "Failed to parse gamma_abc_g1[{index}] from hex! Error: {}",
                    error
                )
            })?;
            gamma_abc_g1.push(ark_ele);
        }

        // Parse the other elements
        let alpha_g1 = parse_type_from_hex(&verification_key_data.alpha_g1)
            .map_err(|error| anyhow!("Failed to parse alpha_g1 from hex! Error: {}", error))?;
        let beta_g2 = parse_type_from_hex(&verification_key_data.beta_g2)
            .map_err(|error| anyhow!("Failed to parse beta_g2 from hex! Error: {}", error))?;
        let gamma_g2 = parse_type_from_hex(&verification_key_data.gamma_g2)
            .map_err(|error| anyhow!("Failed to parse gamma_g2 from hex! Error: {}", error))?;
        let delta_g2 = parse_type_from_hex(&verification_key_data.delta_g2)
            .map_err(|error| anyhow!("Failed to parse delta_g2 from hex! Error: {}", error))?;

        // Construct the VerifyingKey
        let ark_vk = VerifyingKey {
            alpha_g1,
            beta_g2,
            gamma_g2,
            delta_g2,
            gamma_abc_g1,
        };

        Ok(PreparedVerifyingKey::from(ark_vk))
    }
}

/// The data fields of the Groth16VerificationKey resource
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Groth16VerificationKeyData {
    pub alpha_g1: String,
    pub beta_g2: String,
    pub delta_g2: String,
    pub gamma_abc_g1: Vec<String>,
    pub gamma_g2: String,
}

/// Parses a type that implements canonical deserialization from a hex string
fn parse_type_from_hex<T: CanonicalDeserialize>(hex_string: &str) -> Result<T> {
    // Get the raw bytes from the hex string
    let raw_bytes = utils::unhexlify_api_bytes(hex_string)
        .map_err(|error| anyhow!("Failed to unhexlify input hex string! Error: {}", error))?;

    // Deserialize the bytes into the desired type
    T::deserialize_compressed(raw_bytes.as_slice())
        .map_err(|e| anyhow!("Failed to deserialize compressed bytes! Error: {}", e))
}

#[cfg(test)]
mod tests {
    use crate::external_resources::groth16_vk::{
        Groth16VerificationKeyData, OnChainGroth16VerificationKey,
    };
    use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
    use ark_groth16::Groth16;
    use ark_serialize::CanonicalDeserialize;

    #[test]
    fn test_to_ark_pvk() {
        // Create a Groth16 verification key
        let groth16_verification_key = OnChainGroth16VerificationKey {
            r#type: "0x1::keyless_account::Groth16VerificationKey".into(),
            data: Groth16VerificationKeyData {
                alpha_g1: "0xe39cb24154872dbdbbdbc8056c6eb3e6cab3ad82f80ded72ed4c9301c5b3da15".into(),
                beta_g2: "0x9a732e38644f89ad2c7bd629b84d6b81f2e83ca4b3cddfd99c0254e49332861e2fcec4f74545abdd42c8857ff8df6d3f6b3670f930d1d5ba961655ea38ded315".into(),
                delta_g2: "0x04c7b3a2734731369a281424c2bd7af229b92496527fd0a01bfe4a5c01e0a92f256921817b6d6cf040ccd483d81738ac88571b57009f182946e8a88cced03a01".into(),
                gamma_abc_g1: vec![
                    "0x2f4f4bc4acbea0c3bae9e676fb59537e2e46994d5896e286e6fcccc7e14b1b2d".into(),
                    "0x979308443fbac05f6d22a16525c26246e965a9be68e163154f44b20d6b2ddf18".into(),
                ],
                gamma_g2: "0xedf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19".into(),
            },
        };

        // Convert the key to a prepared verifying key
        let prepared_verifying_key = groth16_verification_key
            .to_ark_prepared_verifying_key()
            .unwrap();

        // Construct a proof and public inputs that can be verified with this key
        let proof = ark_groth16::Proof {
            a: G1Affine::deserialize_compressed(hex::decode("fd6ae6c19f7eb7362e420e3e359f3d85c0030b779a815627b805276e45018817").unwrap().as_slice()).unwrap(),
            b: G2Affine::deserialize_compressed(hex::decode("c4af5f1e793653d80009c19637b326f78a46d9d031e9e36a6f87e296ba04e31322af2d8d5c3d4129b6b2f3d222f741ce17145ab62f1b238f3f9e8c88831da297").unwrap().as_slice()).unwrap(),
            c: G1Affine::deserialize_compressed(hex::decode("a856a923cde90ecb6f8955c9627ede579e67b7082431b965d011fca578892096").unwrap().as_slice()).unwrap(),
        };
        let public_input_bytes =
            hex::decode("08aba90163b227d54013b7d1a892b20edf149a23acf810acf78e8baf8e770d11")
                .unwrap();
        let public_inputs =
            vec![Fr::deserialize_compressed(public_input_bytes.as_slice()).unwrap()];

        // Verify the proof using the prepared verifying key and public inputs
        let result =
            Groth16::<Bn254>::verify_proof(&prepared_verifying_key, &proof, &public_inputs);

        // Ensure the verification was successful
        assert_eq!(result, Ok(true));
    }
}
