




use serde::{Deserialize, Serialize};






use std::fs;
use aptos_types::{
    keyless::{
        g1_projective_str_to_affine, 
        g2_projective_str_to_affine
    }
};
use ark_bn254::{Bn254};
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};



#[derive(Serialize, Deserialize, Debug)]
struct RawVK {
    vk_alpha_1: Vec<String>,
    vk_beta_2: Vec<Vec<String>>,
    vk_gamma_2: Vec<Vec<String>>,
    vk_delta_2: Vec<Vec<String>>,
    IC: Vec<Vec<String>>,
}

/// This function uses the decimal uncompressed point serialization which is outputted by circom.
pub fn prepared_vk(vk_file_path: &str) -> PreparedVerifyingKey<Bn254> {
    let raw_vk : RawVK = serde_yaml::from_str(&fs::read_to_string(vk_file_path).expect("Unable to read file")).expect("should parse correctly");

    let alpha_g1 = g1_projective_str_to_affine(
        &raw_vk.vk_alpha_1[0],
        &raw_vk.vk_alpha_1[1],
    )
    .unwrap();

    let beta_g2 = g2_projective_str_to_affine(
        [
            &raw_vk.vk_beta_2[0][0],
            &raw_vk.vk_beta_2[0][1],
        ],
        [
            &raw_vk.vk_beta_2[1][0],
            &raw_vk.vk_beta_2[1][1],
        ],
        )
        .unwrap();

    let gamma_g2 = g2_projective_str_to_affine(
        [
            &raw_vk.vk_gamma_2[0][0],
            &raw_vk.vk_gamma_2[0][1],
        ],
        [
            &raw_vk.vk_gamma_2[1][0],
            &raw_vk.vk_gamma_2[1][1],
        ],
    )
    .unwrap();

    let delta_g2 = g2_projective_str_to_affine(
        [
            &raw_vk.vk_delta_2[0][0],
            &raw_vk.vk_delta_2[0][1],
        ],
        [
            &raw_vk.vk_delta_2[1][0],
            &raw_vk.vk_delta_2[1][1],
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for p in raw_vk.IC {
        gamma_abc_g1.push(
            g1_projective_str_to_affine(
                &p[0],
                &p[1],
                ).unwrap());
    }
    

    let vk = VerifyingKey {
        alpha_g1,
        beta_g2,
        gamma_g2,
        delta_g2,
        gamma_abc_g1,
    };

    PreparedVerifyingKey::from(vk)
}


