// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use ark_bls12_381::Bls12_381;
use ark_groth16::VerifyingKey;
use ark_serialize::CanonicalSerialize;
use handlebars::{to_json, Handlebars};
use std::{collections::BTreeMap, path::PathBuf};

pub const MODULE_TEMPLATE: &str = include_str!("module_template.move");

fn key_bytes_to_string(key: &[u8]) -> String {
    let mut output = "".to_string();
    output += "vector[";
    for b in key.iter() {
        output += format!("{},", b).as_str();
    }

    output += "]";
    return output;
}

pub fn export_move_module(
    verification_key: &VerifyingKey<Bls12_381>,
    output_path: PathBuf,
    name: String,
) -> Result<()> {
    // Add the verifier code to the package.
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("output", MODULE_TEMPLATE)
        .unwrap();

    let mut data = BTreeMap::new();
    macro_rules! key_to_string {
        ($key: expr) => {
            key_bytes_to_string(&{
                let mut writer = vec![];
                $key.serialize_uncompressed(&mut writer).unwrap();
                writer
            })
        };
    }
    data.insert(
        "vk_alpha_g1_bytes",
        to_json(key_to_string!(verification_key.alpha_g1)),
    );
    data.insert(
        "vk_beta_g2_bytes",
        to_json(key_to_string!(verification_key.beta_g2)),
    );
    data.insert(
        "vk_gamma_g2_bytes",
        to_json(key_to_string!(verification_key.gamma_g2)),
    );
    data.insert(
        "vk_delta_g2_bytes",
        to_json(key_to_string!(verification_key.delta_g2)),
    );
    data.insert(
        "vk_uvw_gamma_g1",
        to_json(verification_key.gamma_abc_g1.iter().map(|key| key_to_string!(key)).collect::<Vec<_>>()),
    );
    data.insert("name", to_json(name));

    std::fs::write(&output_path, handlebars.render("output", &data).unwrap()).unwrap();

    Ok(())
}
