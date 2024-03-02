// my understanding of what circuit expects
// <jwt> + "1" + "0"*K + <jwt_bit_len_encoded_for_sha(jwt)> + "0"s until padded to max length for
// this array
//
// python:
// <jwt> + "1" +

pub mod circuit_input_signals;
pub mod config;
pub mod encoding;
pub mod field_check_input;
pub mod public_inputs_hash;
pub mod rsa;
pub mod sha;
pub mod bits;
pub mod types;
pub mod field_parser;
pub mod preprocess;

use self::circuit_input_signals::Padded;

use self::public_inputs_hash::compute_public_inputs_hash;
use crate::input_conversion::circuit_input_signals::CircuitInputSignals;
use crate::input_conversion::encoding::*;
use crate::input_conversion::encoding::JwtParts;
use crate::input_conversion::encoding::UnsignedJwtPartsWithPadding;
use crate::input_conversion::types::Input;

use aptos_crypto::poseidon_bn254;
use aptos_types::jwks::rsa::RSA_JWK;
use aptos_types::keyless::Configuration;
use aptos_types::transaction::authenticator::EphemeralPublicKey;
use ark_bn254::{self, Fr};


use ark_ff::{PrimeField};
use encoding::As64BitLimbs;
use encoding::{FromB64};
use field_check_input::padded_field_check_input_signals;
use hex;

use sha::{jwt_bit_len_binary, compute_sha_padding_without_len, with_sha_padding_bytes};
use tracing::info_span;



use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use super::jwk_fetching;



// TODO highest-impact cleanup tasks:
// 1. Separate types.rs into multiple files (plan in types.rs)
// 2. Separate out encoding via  RequestInput -> Input 
// 3. Rewrite field_check_input.rs


// TODO this works when I have it here, but doesn't when I move it to encoding.rs. Why?
impl FromHex for Fr {
    fn from_hex(s: &str) -> Result<Self> where Self: Sized {
        Ok(Fr::from_le_bytes_mod_order(&hex::decode(&s)?))
    }
}


pub fn derive_circuit_input_signals(
    // TODO: input should not have any hex-encoded anything. Should have conversion from
    // RequestInput also decode all these values. Can even have that function get the jwk.
    // Can also include JwtParts?
    input: Input,
    config: &config::CircuitConfig,
    maybe_jwk: Option<&RSA_JWK>,
) -> Result<(CircuitInputSignals<Padded>, Fr), anyhow::Error> {

  

    // TODO add metrics instead of just printing out elapsed time
    let _start_time = Instant::now();
    let _span = info_span!("Running input conversion");


    let jwt_parts = JwtParts::from_b64(&input.jwt_b64)?;

    let unsigned_jwt_with_padding = with_sha_padding_bytes(&jwt_parts.unsigned_undecoded());
    let signature = jwt_parts.signature()?;

    let header_decoded = jwt_parts.header_decoded()?;
    let header_struct : JwtHeader = serde_json::from_str(&header_decoded)?;
    println!("{:?}", header_decoded);



    let payload_decoded = jwt_parts.payload_decoded()?;
    let payload_struct : JwtPayload = serde_json::from_str(&payload_decoded)?;

    let jwk = match maybe_jwk {
        Some(x) => Arc::new(x.clone()),
        None => jwk_fetching::cached_decoding_key(&payload_struct.iss, &header_struct.kid)?
    };

    // Check the signature verifies.
    jwk.verify_signature(&input.jwt_b64)?;


    // TODO pepper should have a type and should have from_hex method. Can use Pepper from
    // aptos-types?
    // TODO EphemeralPublicKey should have from_hex as well.
    println!("pepper: {}", &input.pepper_fr.to_string());
    println!(
        "payload decoded: {}",
        payload_decoded
    );

    // TODO do this inside compute_public_inputs_hash?
    let temp_pubkey_frs_with_len = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
        &input.epk.to_bytes().as_slice(),
        Configuration::new_for_devnet().max_commited_epk_bytes as usize, // TODO should put this in my local config
    )?;

    let public_inputs_hash = compute_public_inputs_hash(
        &input,
        config,
        input.pepper_fr.clone(),
        &jwt_parts,
        &jwk,
        &temp_pubkey_frs_with_len[..3],
        temp_pubkey_frs_with_len[3],
    )?;

    let epk_blinder_fr = input.epk_blinder_fr;

    let circuit_input_signals = CircuitInputSignals::new()
        // "global" inputs
        .bytes_input("jwt", &unsigned_jwt_with_padding)
        .str_input(
            "jwt_header_with_separator",
            &jwt_parts.header_undecoded_with_dot(),
        )
        .bytes_input(
            "jwt_payload",
            &UnsignedJwtPartsWithPadding::from_b64_bytes_with_padding(
                &unsigned_jwt_with_padding
                ).payload_with_padding()?
        )
        .str_input(
            "jwt_payload_without_sha_padding",
            &jwt_parts.payload_undecoded(),
        )
        .usize_input(
            "header_len_with_separator",
            jwt_parts.header_undecoded_with_dot().len(),
        )
        .usize_input("b64_payload_len", jwt_parts.payload_undecoded().len())
        .usize_input(
            "jwt_num_sha2_blocks",
            unsigned_jwt_with_padding.len() * 8 / 512,
        )
        .bytes_input(
            "jwt_len_bit_encoded",
            &jwt_bit_len_binary(&jwt_parts.unsigned_undecoded()).as_bytes()?
        )
        .bytes_input(
            "padding_without_len",
            &compute_sha_padding_without_len(&jwt_parts.unsigned_undecoded()).as_bytes()?,
        )
        .limbs_input("signature", &signature.as_64bit_limbs())
        .limbs_input("pubkey_modulus", &jwk.as_64bit_limbs())
        .u64_input("exp_date", input.exp_date_secs)
        .u64_input("exp_delta", input.exp_horizon_secs)
        .frs_input("temp_pubkey", &temp_pubkey_frs_with_len[..3])
        .fr_input("temp_pubkey_len", temp_pubkey_frs_with_len[3])
        .fr_input("jwt_randomness", epk_blinder_fr)
        .fr_input("pepper", input.pepper_fr)
        .bool_input("use_extra_field", input.use_extra_field)
        .fr_input("public_inputs_hash", public_inputs_hash)
        // add padding for global inputs
        .pad(&config)?
        // field check inputs
        .merge_foreach(&config.field_check_inputs, |field_check_input_config| {
            padded_field_check_input_signals(
                &payload_decoded,
                &config,
                &field_check_input_config,
                &input.variable_keys,
            )
        })?;

    Ok((circuit_input_signals, public_inputs_hash))
}

pub fn compute_nonce(
    exp_date: u64,
    epk: &EphemeralPublicKey,
    epk_blinder: Fr,
    config: &config::CircuitConfig,
) -> anyhow::Result<Fr> {
    let mut frs = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
        epk.to_bytes().as_slice(),
        config.global_input_max_lengths["temp_pubkey"] * poseidon_bn254::BYTES_PACKED_PER_SCALAR,
    )?;

    frs.push(Fr::from(exp_date));
    frs.push(epk_blinder);

    let nonce_fr = poseidon_bn254::hash_scalars(frs)?;
    Ok(nonce_fr)
}


#[cfg(test)]
mod tests {
    use crate::input_conversion::config::{CircuitConfig, Key};
    use crate::input_conversion::encoding::{FromB64, JwtParts};

    use crate::input_conversion;
    
    use crate::input_conversion::{compute_nonce, derive_circuit_input_signals};
    use crate::input_conversion::types::Input;
    use aptos_crypto::ed25519::Ed25519PublicKey;
    use aptos_crypto::poseidon_bn254;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, encoding_type::EncodingType};
    use aptos_types::jwks::rsa::RSA_JWK;
    use aptos_types::keyless::Configuration;
    use aptos_types::transaction::authenticator::EphemeralPublicKey;
    use ark_bn254;
    
    
    
    use serde_json;
    use serde_yaml;
    use std::collections::HashMap;
    
    use std::fs;
    use std::str::FromStr;

    #[test]
    fn test_epk_packing() {
        let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
            .decode_key(
                "zkid test ephemeral private key",
                "0x76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
                    .as_bytes()
                    .to_vec(),
            )
            .unwrap();
        let epk_unwrapped = Ed25519PublicKey::from(&ephemeral_private_key);
        println!("{}", epk_unwrapped);
        let ephemeral_public_key: EphemeralPublicKey = EphemeralPublicKey::ed25519(epk_unwrapped);

        let temp_pubkey_frs_with_len = poseidon_bn254::pad_and_pack_bytes_to_scalars_with_len(
            ephemeral_public_key.to_bytes().as_slice(),
            Configuration::new_for_testing().max_commited_epk_bytes as usize, // TODO should use my own thing here
        )
        .unwrap();

        let temp_pubkey_frs = &temp_pubkey_frs_with_len[0..3];

        let temp_pubkey_0 =
            "242984842061174104272170180221318235913385474778206477109637294427650138112";
        let temp_pubkey_1 = "4497911";
        let temp_pubkey_2 = "0";
        let _temp_pubkey_len = "34";

        println!(
            "pubkey frs: {} {} {}",
            temp_pubkey_frs[0].to_string(),
            temp_pubkey_frs[1].to_string(),
            temp_pubkey_frs[2].to_string()
        );
        assert!(temp_pubkey_frs[0] == ark_bn254::Fr::from_str(temp_pubkey_0).unwrap());
        assert!(temp_pubkey_frs[1] == ark_bn254::Fr::from_str(temp_pubkey_1).unwrap());
        assert!(temp_pubkey_frs[2] == ark_bn254::Fr::from_str(temp_pubkey_2).unwrap());
    }

    #[test]
    fn test_convert() {
        let jwt_b64 = "eyJhbGciOiJSUzI1NiIsImtpZCI6InRlc3RfandrIiwidHlwIjoiSldUIn0.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhenAiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJhdWQiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJzdWIiOiIxMTM5OTAzMDcwODI4OTk3MTg3NzUiLCJoZCI6ImFwdG9zbGFicy5jb20iLCJlbWFpbCI6Im1pY2hhZWxAYXB0b3NsYWJzLmNvbSIsImVtYWlsX3ZlcmlmaWVkIjp0cnVlLCJhdF9oYXNoIjoiYnhJRVN1STU5SW9aYjVhbENBU3FCZyIsIm5hbWUiOiJNaWNoYWVsIFN0cmFrYSIsInBpY3R1cmUiOiJodHRwczovL2xoMy5nb29nbGV1c2VyY29udGVudC5jb20vYS9BQ2c4b2NKdlk0a1ZVQlJ0THhlMUlxS1dMNWk3dEJESnpGcDlZdVdWWE16d1BwYnM9czk2LWMiLCJnaXZlbl9uYW1lIjoiTWljaGFlbCIsImZhbWlseV9uYW1lIjoiU3RyYWthIiwibG9jYWxlIjoiZW4iLCJpYXQiOjE3MDAyNTU5NDQsImV4cCI6MjcwMDI1OTU0NCwibm9uY2UiOiI5Mzc5OTY2MjUyMjQ4MzE1NTY1NTA5NzkwNjEzNDM5OTAyMDA1MTU4ODcxODE1NzA4ODczNjMyNDMxNjk4MTkzNDIxNzk1MDMzNDk4In0.JJNqnxZ_CbJm5htLRy9iR9OYFlKXB2ZyRa41HcS5PevwLWgGYS8co3WGwd312712kNZ8t8JXY651VW5YT57-BIVWTzPq4GhIXnS4nGc12IlNJtn5tmgIAeOfUVsLlITdu8jvdGp5lU3fJDCqlyczeFFhnbZ8irNHRr206hxrwLOMvMKq9VH4iMWl3HdDseJdKGNC-HBJ1U9ik6klAd4_pv9bfXzclpnEfLebr9RgITf7sBjHh2n-0k-EIZhxWra37EgU2sTG5oU1hkYbaLKwj8ZZYIbM4CNBlaKq__iE9tLZ2N_mRMG2oYcn7WlTiU2DOydzPUcSrO4jPU3PNgDpjQ";

        let ephemeral_private_key: Ed25519PrivateKey = EncodingType::Hex
            .decode_key(
                "zkid test ephemeral private key",
                "0x76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1aa836efcc8b770dc7"
                    .as_bytes()
                    .to_vec(),
            )
            .unwrap();
        let ephemeral_public_key_unwrapped: Ed25519PublicKey =
            Ed25519PublicKey::from(&ephemeral_private_key);
        let epk = EphemeralPublicKey::ed25519(ephemeral_public_key_unwrapped);
        let epk_blinder = ark_bn254::Fr::from_str("42").unwrap();

        let input = Input {
            jwt_b64: jwt_b64.into(),
            epk: epk,
            epk_blinder_fr: epk_blinder,
            exp_date_secs: 1900255944,
            pepper_fr: ark_bn254::Fr::from_str("76").unwrap(),
            variable_keys: HashMap::from([
                (String::from("uid"), String::from("sub")),
                (String::from("extra"), String::from("family_name")),
            ]),
            use_extra_field: true,
            exp_horizon_secs: 100255944,
        };


        let expected_json: serde_json::Value = serde_json::from_str(
            &fs::read_to_string("tests/input.json").expect("Unable to read file"),
        )
        .expect("should parse correctly");

        let config: CircuitConfig = serde_yaml::from_str(
            &fs::read_to_string("conversion_config.yml").expect("Unable to read file"),
        )
        .expect("should parse correctly");
        //        println!("{}", serde_yaml::to_string(&config).unwrap());
        //
        //
        let jwt_parts = JwtParts::from_b64(&input.jwt_b64).unwrap();
        let _payload_decoded = jwt_parts.payload_decoded().unwrap();
        let _computed_nonce = compute_nonce(input.exp_date_secs, &input.epk, epk_blinder, &config).unwrap();
        //let parsed_nonce = parse_field(&Ascii::from(payload_decoded.as_str()), "nonce").unwrap();
        //assert!(computed_nonce.to_string() == parsed_nonce.value);

        let (signals, _) = derive_circuit_input_signals(
            input,
            &config,
            Some(&RSA_JWK::new_256_aqab(
                input_conversion::michael_pk_kid_str,
                input_conversion::michael_pk_mod_str,
            )),
        )
        .expect("should convert successfully");
        let converted = signals.to_json_value();

        assert!(converted["jwt"] == expected_json["jwt"]);
        assert!(
            converted["jwt_header_with_separator"] == expected_json["jwt_header_with_separator"]
        );
        assert!(converted["jwt_payload"] == expected_json["jwt_payload"]);
        assert!(
            converted["jwt_payload_without_sha_padding"]
                == expected_json["jwt_payload_without_sha_padding"]
        );
        assert!(
            converted["header_len_with_separator"] == expected_json["header_len_with_separator"]
        );
        assert!(converted["b64_payload_len"] == expected_json["b64_payload_len"]);
        assert!(converted["jwt_num_sha2_blocks"] == expected_json["jwt_num_sha2_blocks"]);
        assert!(converted["jwt_len_bit_encoded"] == expected_json["jwt_len_bit_encoded"]);
        assert!(converted["padding_without_len"] == expected_json["padding_without_len"]);
        assert!(converted["pubkey_modulus"] == expected_json["pubkey_modulus"]);
        assert!(converted["exp_date"] == expected_json["exp_date"]);
        assert!(converted["exp_delta"] == expected_json["exp_delta"]);
        assert!(converted["temp_pubkey"] == expected_json["temp_pubkey"]);
        assert!(converted["temp_pubkey_len"] == expected_json["temp_pubkey_len"]);
        assert!(converted["jwt_randomness"] == expected_json["jwt_randomness"]);
        assert!(converted["pepper"] == expected_json["pepper"]);
        //assert!(converted["public_inputs_hash"] == expected_json["public_inputs_hash"]);

        let mut keys = converted
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<&String>>();
        let mut expected_keys = expected_json
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<&String>>();
        keys.sort();
        expected_keys.sort();

        println!("{:?}", keys);
        println!("{:?}", expected_keys);


        // TODO this should go in field_check_input.rs
        for field_config in &config.field_check_inputs {
            let name = &field_config.circuit_input_signal_prefix;
            println!("{}", name);
            assert!(
                converted[String::from(name) + "_field"]
                    == expected_json[String::from(name) + "_field"]
            );
            println!(
                "actual {} field_len: {}",
                name,
                converted[String::from(name) + "_field_len"]
            );
            println!(
                "expected {} field_len: {}",
                name,
                expected_json[String::from(name) + "_field_len"]
            );
            assert!(
                converted[String::from(name) + "_field_len"]
                    == expected_json[String::from(name) + "_field_len"]
            );
            assert!(
                converted[String::from(name) + "_index"]
                    == expected_json[String::from(name) + "_index"]
            );
            if field_config.has_value_inputs {
                if let Key::Variable = field_config.jwt_key {
                    assert!(
                        converted[String::from(name) + "_name_len"]
                            == expected_json[String::from(name) + "_name_len"]
                    );
                } else {
                }

                assert!(
                    converted[String::from(name) + "_colon_index"]
                        == expected_json[String::from(name) + "_colon_index"]
                );
                assert!(
                    converted[String::from(name) + "_name"]
                        == expected_json[String::from(name) + "_name"]
                );
                assert!(
                    converted[String::from(name) + "_value_index"]
                        == expected_json[String::from(name) + "_value_index"]
                );

                if name != "aud" {
                    println!("{} {}", converted[String::from(name) + "_value_len"],
                            expected_json[String::from(name) + "_value_len"]);
                    assert!(
                        converted[String::from(name) + "_value_len"]
                            == expected_json[String::from(name) + "_value_len"]
                    );
                    assert!(
                        converted[String::from(name) + "_value"]
                            == expected_json[String::from(name) + "_value"]
                    );
                } else {
                    assert!(
                        converted["private_aud_value_len"]
                            == expected_json["private_aud_value_len"]
                    );
                    assert!(converted["private_aud_value"] == expected_json["private_aud_value"]);
                    assert!(
                        converted["override_aud_value_len"]
                            == expected_json["override_aud_value_len"]
                    );
                    assert!(converted["override_aud_value"] == expected_json["override_aud_value"]);
                }
            }
        }

        assert!(converted == expected_json);
    }
}
