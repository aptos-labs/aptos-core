// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_infallible::duration_since_epoch;
use aptos_keyless_pepper_common::{
    account_recovery_db::AccountRecoveryDbEntry,
    jwt,
    vuf::{self, VUF},
    PepperInput, PepperRequest, PepperResponse, PepperV0VufPubKey, SignatureResponse,
};
use aptos_types::{
    keyless::{Configuration, OpenIdSig},
    transaction::authenticator::EphemeralPublicKey,
};
use ark_serialize::CanonicalDeserialize;
use firestore::{path, paths, FirestoreDb, FirestoreDbOptions};
use reqwest::StatusCode;
use std::{fs, io::stdin};

const TEST_JWT: &str = "eyJhbGciOiJSUzI1NiIsImtpZCI6ImUxYjkzYzY0MDE0NGI4NGJkMDViZjI5NmQ2NzI2MmI2YmM2MWE0ODciLCJ0eXAiOiJKV1QifQ.eyJpc3MiOiJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20iLCJhenAiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJhdWQiOiI0MDc0MDg3MTgxOTIuYXBwcy5nb29nbGV1c2VyY29udGVudC5jb20iLCJzdWIiOiIxMTE2Mjc3NzI0NjA3NTIzNDIzMTIiLCJhdF9oYXNoIjoiaG5OWHFJVTZ3dWFPYlVqR05lRVhGQSIsIm5vbmNlIjoiNzQyMDQxODMxNDYwMDk1MDM0MTU3NzQ0MzEzMzY0MTU4OTk0NzYwNTExMjc1MDEwNDIyNjY5MDY3NTc3OTY3NTIyNDAwNjA0OTI0NCIsIm5hbWUiOiJPbGl2ZXIgSGUiLCJnaXZlbl9uYW1lIjoiT2xpdmVyIiwiZmFtaWx5X25hbWUiOiJIZSIsImlhdCI6MTcxNDQ0MTc4MywiZXhwIjoxNzE0NDQ1MzgzfQ.iNeVzp4BTQj2I_WH6UaUOfUBV4Q_wUriV7jWkh1fUqTPSs30jMMSjEDZml8lQ_NUIpivnGvfEHt_rF9rlrsuRur9pTVKRRKhJUNf5avrAujvLzrz-bwdgKXtTY_nmYisNNNQwmFIVP004ICois4DHD7EmO8PI88CzSzdDbl9qAIoxOP3JRKRwU05wK5qkGz6FpYzTYiG50lQCybSzzUN5Lws49ANCAOZiROG5lmszOW41mAbFSd6MUX469uvyMA2ZZ5av9ArKricHJPutGtLoOSWpzKQ_mlCzofVs5tHoMhGgcOFKuhnEVdY4J7TdcV6pZv9Ih5F8MX3-Wz9Iz9O4w";

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
    let vuf_pub_key_url = format!("{url}/v0/vuf-pub-key");
    let fetch_url = format!("{url}/v0/fetch");
    let sig_url = format!("{url}/v0/signature");
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
    let epk_expiry_time_secs = duration_since_epoch().as_secs() + 3600;
    println!("expiry_time_sec={}", epk_expiry_time_secs);

    let esk_bytes =
        hex::decode("1111111111111111111111111111111111111111111111111111111111111111").unwrap();
    let serialized: &[u8] = esk_bytes.as_slice();
    let esk = Ed25519PrivateKey::try_from(serialized).unwrap();
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

    let pepper_request = PepperRequest {
        jwt: jwt.clone(),
        epk,
        exp_date_secs: epk_expiry_time_secs,
        uid_key: None,
        epk_blinder: blinder.to_vec(),
        derivation_path: None,
    };
    println!();
    println!(
        "Request pepper with a POST to {} and the body being {}",
        fetch_url,
        serde_json::to_string_pretty(&pepper_request).unwrap()
    );
    let pepper_raw_response = client
        .post(fetch_url)
        .json(&pepper_request)
        .send()
        .await
        .unwrap();
    assert_eq!(StatusCode::OK, pepper_raw_response.status());
    let pepper_response = pepper_raw_response.json::<PepperResponse>().await.unwrap();
    println!();
    println!(
        "pepper_service_response={}",
        serde_json::to_string_pretty(&pepper_response).unwrap()
    );
    let PepperResponse { pepper, address } = pepper_response;

    let signature_raw_response = client
        .post(sig_url)
        .json(&pepper_request)
        .send()
        .await
        .unwrap();
    assert_eq!(StatusCode::OK, signature_raw_response.status());
    let signature_response = signature_raw_response
        .json::<SignatureResponse>()
        .await
        .unwrap();
    println!(
        "signature_response={}",
        serde_json::to_string_pretty(&signature_response).unwrap()
    );
    let SignatureResponse { signature } = signature_response;
    println!("signature={:?}", hex::encode(signature.clone()));
    println!("pepper={:?}", hex::encode(pepper.clone()));
    println!("address={:?}", hex::encode(address.clone()));
    let claims = jwt::parse(jwt.as_str()).unwrap();
    println!();
    println!("Verify the pepper against the server's verification key, part of the JWT, and the actual aud.");
    let pepper_input = PepperInput {
        iss: claims.claims.iss.clone(),
        uid_key: "sub".to_string(),
        uid_val: claims.claims.sub.clone(),
        aud: claims.claims.aud.clone(),
    };
    let pepper_input_bytes = bcs::to_bytes(&pepper_input).unwrap();
    vuf::bls12381_g1_bls::Bls12381G1Bls::verify(&vuf_pk, &pepper_input_bytes, &signature, &[])
        .unwrap();
    println!("Pepper verification succeeded!");

    println!("Checking firestore records.");
    let google_project_id = std::env::var("PROJECT_ID").unwrap();
    let database_id = std::env::var("DATABASE_ID").unwrap();
    let options = FirestoreDbOptions {
        google_project_id,
        database_id,
        max_retries: 1,
        firebase_api_url: None,
    };
    let db = FirestoreDb::with_options(options).await.unwrap();
    let docs: Vec<AccountRecoveryDbEntry> = db
        .fluent()
        .select()
        .fields(paths!(AccountRecoveryDbEntry::{iss, aud, uid_key, uid_val, last_request_unix_ms, first_request_unix_ms_minus_1q, num_requests}))
        .from("accounts")
        .filter(|q| {
            q.for_all([
                q.field(path!(AccountRecoveryDbEntry::iss))
                    .eq(pepper_input.iss.clone()),
                q.field(path!(AccountRecoveryDbEntry::aud))
                    .eq(pepper_input.aud.clone()),
                q.field(path!(AccountRecoveryDbEntry::uid_key))
                    .eq(pepper_input.uid_key.clone()),
                q.field(path!(AccountRecoveryDbEntry::uid_val))
                    .eq(pepper_input.uid_val.clone()),
            ])
        })
        .obj()
        .query()
        .await
        .unwrap();
    println!("docs={docs:?}");
    assert_eq!(1, docs.len());
}
