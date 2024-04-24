// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! These constants are from commit 125522b4b226f8ece3e3162cecfefe915d13bc30 of keyless-circuit.

use crate::keyless::bn254_circom::{g1_projective_str_to_affine, g2_projective_str_to_affine};
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
        "20491192805390485299153009773594534940189261866228447918068658471970481763042",
        "9383485363053290200918347156157836566562967994039712273449902621266178545958",
    )
    .unwrap();

    let beta_g2 = g2_projective_str_to_affine(
        [
            "6375614351688725206403948262868962793625744043794305715222011528459656738731",
            "4252822878758300859123897981450591353533073413197771768651442665752259397132",
        ],
        [
            "10505242626370262277552901082094356697409835680220590971873171140371331206856",
            "21847035105528745403288232691147584728191162732299865338377159692350059136679",
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
            "19759886250806183187785579505109257837989251596255610913102572077808842056375",
            "8515569072948108462120402914801299810016610043704833841603450087200707784492",
        )
        .unwrap(),
        g1_projective_str_to_affine(
            "18250059095913215666541561118844673017538035392793529003420365565251085504261",
            "21846936675713878002567053788450833465715833259428778772043736890983365407823",
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
