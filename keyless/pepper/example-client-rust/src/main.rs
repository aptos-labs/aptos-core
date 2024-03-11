// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_keyless_pepper_common::{jwt, vuf, vuf::VUF, PepperInput, PepperRequest, PepperResponse, PepperV0VufPubKey, PepperResponseV1, PepperRequestV1};
use aptos_types::{
    keyless::{test_utils::get_sample_esk, Configuration, OpenIdSig},
    transaction::authenticator::EphemeralPublicKey,
};
use ark_serialize::CanonicalDeserialize;
use reqwest::StatusCode;
use std::{
    fs,
    io::stdin,
    time::{SystemTime, UNIX_EPOCH},
};
use aptos_crypto::asymmetric_encryption::AsymmetricEncryption;
use aptos_crypto::asymmetric_encryption::elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm;

const TEST_JWT: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6IjU1YzE4OGE4MzU0NmZjMTg4ZTUxNTc2YmE3MjgzNmUwNjAwZThiNzMiLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhenAiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJhdWQiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJzdWIiOiIxMTE2Mjc3NzI0NjA3NTIzNDIzMTIiLCJhdF9oYXNoIjoiOHNGRHVXTXlURkVDNWl5Q1RRY2F3dyIsIm5vbmNlIjoiMTE3NjI4MjY1NzkyNTY5MTUyNDYzNzU5MTE3MjkyNjg5Nzk3NzQzNzI2ODUwNjI5ODI2NDYxMDYxMjkxMDAzMjE1OTk2MjczMTgxNSIsIm5hbWUiOiJPbGl2ZXIgSGUiLCJnaXZlbl9uYW1lIjoiT2xpdmVyIiwiZmFtaWx5X25hbWUiOiJIZSIsImxvY2FsZSI6ImVuIiwiaWF0IjoxNzA4OTIwNzY3LCJleHAiOjE3MDg5MjQzNjd9.j6qdaQDaUcD5uhbTp3jWfpLlSACkVLlYQZvKZG2rrmLJOAmcz5ADN8EtIR_JHuTUWvciDOmEdF1w2fv7MseNmKPEgzrkASsfYmk0H50wVn1R9lGfXCkklr3V_hzIHA7jSFw0c1_--epHjBa7Uxlfe0xAV3pnbl7hmFrmin_HFAfw0_xQP-ohsjsnhxiviDgESychRSpwJZG_HBm-AHGDJ3lNTF2fYdsL1Vr8CYogBNQG_oqTLhipEiGS01eWjw7s02MydsKFIA3WhYu5HxUg8223iVdGq7dBMM8y6gFncabBEOHRnaZ1w_5jKlmX-m7bus7bHTDbAzjkmxNFqD-pPw";

fn read_line_from_stdin() -> String {
    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();
    line
}

fn get_pepper_service_url() -> String {
    match std::env::var("OIDB_PEPPER_TEST_CLIENT__SERVICE_URL") {
        Ok(val) => {
            println!();
            println!(
                "Pepper service url found from envvar OIDB_PEPPER_TEST_CLIENT__SERVICE_URL: {}",
                val
            );
            val
        },
        Err(_) => {
            println!();
            println!("Pepper service url not found from envvar OIDB_PEPPER_SERVICE_URL.");
            println!("Enter the URL of the targeted pepper service deployment (default: http://localhost:8000):");
            let raw = read_line_from_stdin().trim().to_string();
            if raw.is_empty() {
                "http://localhost:8000".to_string()
            } else {
                raw
            }
        },
    }
}

fn get_jwt_or_path() -> String {
    println!();
    println!(
        "Enter the JWT token (defaults to test token), or a text file path that contains the JWT:"
    );
    let user_input = read_line_from_stdin().trim().to_string();
    if !user_input.is_empty() {
        user_input
    } else {
        println!("Using the test JWT token");
        TEST_JWT.to_string()
    }
}

#[tokio::main]
async fn main() {
    println!();
    println!("Starting an interaction with aptos-oidb-pepper-service.");
    let url = get_pepper_service_url();
    println!();
    let vuf_pub_key_url = format!("{url}/v1/vuf-pub-key");
    let fetch_url = format!("{url}/v1/fetch");
    println!();
    println!(
        "Action 1: fetch its verification key with a GET request to {}",
        vuf_pub_key_url
    );
    let client = reqwest::Client::new();
    let response = client
        .get(vuf_pub_key_url)
        .send()
        .await
        .unwrap()
        .json::<PepperV0VufPubKey>()
        .await
        .unwrap();
    println!();
    println!(
        "response_json={}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    let PepperV0VufPubKey { public_key: vuf_pk } = response;
    let vuf_pk: ark_bls12_381::G2Projective =
        ark_bls12_381::G2Affine::deserialize_compressed(vuf_pk.as_slice())
            .unwrap()
            .into();

    println!();
    println!("Action 3: generate some random bytes as a blinder.");
    let blinder: [u8; 31] = [0u8; 31];
    println!("blinder_hexlified={}", hex::encode(blinder));

    println!();
    println!("Action 4: decide an expiry unix time.");
    let epk_expiry_time_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 86400;
    println!("expiry_time_sec={}", epk_expiry_time_secs);

    let esk = get_sample_esk();
    let epk = EphemeralPublicKey::ed25519(Ed25519PublicKey::from(&esk));
    println!("esk_hexlified={}", hex::encode(esk.to_bytes()));
    println!("epk_hexlified={}", hex::encode(epk.to_bytes()));

    println!();
    println!("Action 5: compute nonce.");
    let nonce_str = OpenIdSig::reconstruct_oauth_nonce(
        blinder.as_slice(),
        epk_expiry_time_secs,
        &epk,
        &Configuration::new_for_devnet(),
    )
    .unwrap();
    println!("nonce_string={}", nonce_str);
    println!();
    println!("Action 6: request a JWT with this nonce. Below are generated example that uses Google OAuth 2.0 Playground:");
    println!("6.1: Go to https://accounts.google.com/o/oauth2/v2/auth/oauthchooseaccount?redirect_uri=https%3A%2F%2Fdevelopers.google.com%2Foauthplayground&prompt=consent&response_type=code&client_id=407408718192.apps.googleusercontent.com&scope=profile&access_type=offline&service=lso&o2v=2&theme=glif&flowName=GeneralOAuthFlow&nonce={nonce_str}");
    println!("6.2: Sign in as requested by the web UI");
    println!("6.3: Once you are signed in to 'OAuth 2.0 Playground' and see a blue button called 'Exchange authorization code for tokens', click it");
    println!("6.4: You should see some response showing up. Take the value of the field 'id_token' (exclude the double-quotes) and save it to a file");
    let jwt_or_path = get_jwt_or_path();
    let jwt = match fs::read_to_string(jwt_or_path.clone()) {
        Ok(raw_str) => raw_str.trim().to_string(),
        Err(_) => jwt_or_path,
    };

    let pepper_request = PepperRequestV1 {
        jwt: jwt.clone(),
        epk,
        exp_date_secs: epk_expiry_time_secs,
        uid_key: None,
        epk_blinder: blinder.to_vec(),
        aud_override: None,
    };
    println!();
    println!(
        "Request pepper with a POST to {} and the body being {}",
        fetch_url,
        serde_json::to_string_pretty(&pepper_request).unwrap()
    );
    let raw_response = client
        .post(fetch_url)
        .json(&pepper_request)
        .send()
        .await
        .unwrap();
    assert_eq!(StatusCode::OK, raw_response.status());
    let pepper_response = raw_response.json::<PepperResponseV1>().await.unwrap();
    println!();
    println!(
        "pepper_service_response={}",
        serde_json::to_string_pretty(&pepper_response).unwrap()
    );
    let PepperResponseV1 { signature_encrypted } = pepper_response;
    let curve25519_scalar_bytes = esk.derive_scalar().as_bytes().to_vec();
    let pepper = ElGamalCurve25519Aes256Gcm::dec(&curve25519_scalar_bytes, &signature_encrypted).unwrap();
    println!();
    println!("pepper={:?}", pepper);
    let claims = jwt::parse(jwt.as_str()).unwrap();
    println!();
    println!("Verify the pepper against the server's verification key and part of the JWT.");
    let pepper_input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: "sub".to_string(),
        uid_val: claims.claims.sub.clone(),
        aud: claims.claims.aud.clone(),
    };
    let pepper_input_bytes = bcs::to_bytes(&pepper_input).unwrap();
    vuf::bls12381_g1_bls::Bls12381G1Bls::verify(&vuf_pk, &pepper_input_bytes, &pepper, &[])
        .unwrap();
    println!();
    println!("Pepper verification succeeded!");
}
