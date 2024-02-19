// Copyright © Aptos Foundation

//! These constants are from commit 125522b4b226f8ece3e3162cecfefe915d13bc30 of zkid-circuit.

use crate::zkid::bn254_circom::{g1_projective_str_to_affine, g2_projective_str_to_affine};
use aptos_crypto::poseidon_bn254;
use ark_bn254::Bn254;
use ark_groth16::{PreparedVerifyingKey, VerifyingKey};

pub(crate) const MAX_AUD_VAL_BYTES: usize = 120;
pub(crate) const MAX_UID_KEY_BYTES: usize = 30;
pub(crate) const MAX_UID_VAL_BYTES: usize = 330;
pub(crate) const MAX_ISS_VAL_BYTES: u16 = 120;
pub(crate) const MAX_EXTRA_FIELD_BYTES: u16 = 350;
pub(crate) const MAX_JWT_HEADER_B64_BYTES: u32 = 300;

/// This constant is not explicitly defined in the circom template, but only implicitly in the way
/// we hash the EPK.
pub(crate) const MAX_COMMITED_EPK_BYTES: u16 = 3 * poseidon_bn254::BYTES_PACKED_PER_SCALAR as u16;

/// This function uses the decimal uncompressed point serialization which is outputted by circom.
/// https://github.com/aptos-labs/devnet-groth16-keys/commit/02e5675f46ce97f8b61a4638e7a0aaeaa4351f76
pub fn devnet_prepared_vk() -> PreparedVerifyingKey<Bn254> {
    // Convert the projective points to affine.
    let alpha_g1 = g1_projective_str_to_affine(
        "16672231080302629756836614130913173861541009360974119524782950408048375831661",
        "1076145001163048025135533382088266750240489485046298539187659509488738517245",
    )
    .unwrap();

    let beta_g2 = g2_projective_str_to_affine(
        [
            "1125365732643211423779651913319958385653115422366520671538751860820509133538",
            "10055196097002324305342942912758079446356594743098794928675544207400347950287",
        ],
        [
            "10879716754714953827605171295191459580695363989155343984818520267224463075503",
            "440220374146936557739765173414663598678359360031905981547938788314460390904",
        ],
    )
    .unwrap();

    let gamma_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let delta_g2 = g2_projective_str_to_affine(
        [
            "10857046999023057135944570762232829481370756359578518086990519993285655852781",
            "11559732032986387107991004021392285783925812861821192530917403151452391805634",
        ],
        [
            "8495653923123431417604973247489272438418190587263600148770280649306958101930",
            "4082367875863433681332203403145435568316851327593401208105741076214120093531",
        ],
    )
    .unwrap();

    let mut gamma_abc_g1 = Vec::new();
    for points in [
        g1_projective_str_to_affine(
            "333957087685714773491410343905674131693317845924221586503521553512853800005",
            "16794842110397433586916934076838854067112427849394773076676106408631114267154",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "14679941092573826838949544937315479399329040741655244517938404383938168565228",
            "19977040285201397592140173066949293223501504328707794673737757867503037033174",
        )
        .unwrap(),
    ] {
        gamma_abc_g1.push(points);
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
