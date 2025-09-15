// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{external_resources::resource_fetcher::CachedResource, utils};
use anyhow::{anyhow, Result};
use ark_bn254::{Bn254, G1Affine, G2Affine};
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};
use ark_serialize::CanonicalDeserialize;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct VKeyData {
    pub alpha_g1: String,
    pub beta_g2: String,
    pub delta_g2: String,
    pub gamma_abc_g1: Vec<String>,
    pub gamma_g2: String,
}

/// On-chain representation of a VK.
///
/// https://fullnode.testnet.aptoslabs.com/v1/accounts/0x1/resource/0x1::keyless_account::Groth16VerificationKey
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct OnChainGroth16VerificationKey {
    /// Some type info returned by node API.
    pub r#type: String,
    pub data: VKeyData,
}

impl OnChainGroth16VerificationKey {
    pub fn to_ark_pvk(&self) -> Result<PreparedVerifyingKey<Bn254>> {
        let mut gamma_abc_g1 = Vec::with_capacity(self.data.gamma_abc_g1.len());
        for (idx, onchain_ele) in self.data.gamma_abc_g1.iter().enumerate() {
            let ark_ele = g1_from_api_repr(onchain_ele).map_err(|e| {
                anyhow!("to_ark_pvk() failed with gamma_abc_g1[{idx}] convert err: {e}")
            })?;
            gamma_abc_g1.push(ark_ele);
        }
        let ark_vk = VerifyingKey {
            alpha_g1: g1_from_api_repr(&self.data.alpha_g1)
                .map_err(|e| anyhow!("to_ark_pvk() failed with alpha_g1 convert err: {e}"))?,
            beta_g2: g2_from_api_repr(&self.data.beta_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with beta_g2 convert err: {e}"))?,
            gamma_g2: g2_from_api_repr(&self.data.gamma_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with gamma_g2 convert err: {e}"))?,
            delta_g2: g2_from_api_repr(&self.data.delta_g2)
                .map_err(|e| anyhow!("to_ark_pvk() failed with delta_g2 convert err: {e}"))?,
            gamma_abc_g1,
        };
        Ok(PreparedVerifyingKey::from(ark_vk))
    }
}

fn g1_from_api_repr(api_repr: &str) -> Result<G1Affine> {
    let bytes = utils::unhexlify_api_bytes(api_repr)
        .map_err(|e| anyhow!("g1_from_api_repr() failed with unhex err: {e}"))?;
    let ret = G1Affine::deserialize_compressed(bytes.as_slice())
        .map_err(|e| anyhow!("g1_from_api_repr() failed with g1 deser err: {e}"))?;
    Ok(ret)
}

fn g2_from_api_repr(api_repr: &str) -> Result<G2Affine> {
    let bytes = utils::unhexlify_api_bytes(api_repr)
        .map_err(|e| anyhow!("g2_from_api_repr() failed with unhex err: {e}"))?;
    let ret = G2Affine::deserialize_compressed(bytes.as_slice())
        .map_err(|e| anyhow!("g2_from_api_repr() failed with g2 deser err: {e}"))?;
    Ok(ret)
}

impl CachedResource for OnChainGroth16VerificationKey {
    fn resource_name() -> String {
        "OnChainGroth16VerificationKey".to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::external_resources::groth16_vk::{OnChainGroth16VerificationKey, VKeyData};
    use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
    use ark_groth16::Groth16;
    use ark_serialize::CanonicalDeserialize;

    #[test]
    fn test_to_ark_pvk() {
        let api_repr = OnChainGroth16VerificationKey {
            r#type: "0x1::keyless_account::Groth16VerificationKey".to_string(),
            data: VKeyData {
                alpha_g1: "0xe39cb24154872dbdbbdbc8056c6eb3e6cab3ad82f80ded72ed4c9301c5b3da15".to_string(),
                beta_g2: "0x9a732e38644f89ad2c7bd629b84d6b81f2e83ca4b3cddfd99c0254e49332861e2fcec4f74545abdd42c8857ff8df6d3f6b3670f930d1d5ba961655ea38ded315".to_string(),
                delta_g2: "0x04c7b3a2734731369a281424c2bd7af229b92496527fd0a01bfe4a5c01e0a92f256921817b6d6cf040ccd483d81738ac88571b57009f182946e8a88cced03a01".to_string(),
                gamma_abc_g1: vec![
                    "0x2f4f4bc4acbea0c3bae9e676fb59537e2e46994d5896e286e6fcccc7e14b1b2d".to_string(),
                    "0x979308443fbac05f6d22a16525c26246e965a9be68e163154f44b20d6b2ddf18".to_string(),
                ],
                gamma_g2: "0xedf692d95cbdde46ddda5ef7d422436779445c5e66006a42761e1f12efde0018c212f3aeb785e49712e7a9353349aaf1255dfb31b7bf60723a480d9293938e19".to_string(),
            },
        };

        let ark_pvk = api_repr.to_ark_pvk().unwrap();
        let proof = ark_groth16::Proof {
            a: G1Affine::deserialize_compressed(hex::decode("fd6ae6c19f7eb7362e420e3e359f3d85c0030b779a815627b805276e45018817").unwrap().as_slice()).unwrap(),
            b: G2Affine::deserialize_compressed(hex::decode("c4af5f1e793653d80009c19637b326f78a46d9d031e9e36a6f87e296ba04e31322af2d8d5c3d4129b6b2f3d222f741ce17145ab62f1b238f3f9e8c88831da297").unwrap().as_slice()).unwrap(),
            c: G1Affine::deserialize_compressed(hex::decode("a856a923cde90ecb6f8955c9627ede579e67b7082431b965d011fca578892096").unwrap().as_slice()).unwrap(),
        };
        let public_inputs = vec![Fr::deserialize_compressed(
            hex::decode("08aba90163b227d54013b7d1a892b20edf149a23acf810acf78e8baf8e770d11")
                .unwrap()
                .as_slice(),
        )
        .unwrap()];
        assert!(Groth16::<Bn254>::verify_proof(&ark_pvk, &proof, &public_inputs).unwrap());
    }
}
