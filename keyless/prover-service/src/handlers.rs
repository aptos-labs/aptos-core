// Copyright © Aptos Foundation

use crate::{
    api::{FromFr, PoseidonHash, ProverServerResponse, RequestInput},
    config::*,
    error,
    input_conversion::{config::CircuitConfig, derive_circuit_input_signals, preprocess},
    witness_gen::witness_gen,
};
use anyhow::{anyhow, Result};
use aptos_crypto::{
    ed25519::Ed25519PublicKey,
    traits::{Signature, SigningKey},
};
use aptos_types::keyless::{G1Bytes, G2Bytes, Groth16Zkp, Groth16ZkpAndStatement};
use axum::{extract::State, http::StatusCode, Json};
use serde_json::value::Value;
use std::{
    fs,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Instant,
};
use tracing::info_span;

pub async fn prove_handler(
    State(state): State<Arc<Mutex<ProverServerState>>>,
    Json(body): Json<RequestInput>,
) -> Result<Json<ProverServerResponse>, (StatusCode, Json<ProverServerResponse>)> {
    let start_time = Instant::now();
    let span = info_span!("Handling /prove");
    let _enter = span.enter();

    let mut state_unlocked = state.lock().expect("Can't lock mutex");
    state_unlocked.metrics.request_counter.inc();

    // TODO why tf am I reading this from a file every request lol
    let config: CircuitConfig = serde_yaml::from_str(
        &fs::read_to_string("conversion_config.yml").expect("Unable to read file"),
    )
    .expect("should parse correctly");

    let input = preprocess::decode_and_add_jwk(body).map_err(|e| {
        error::make_error(
            e,
            StatusCode::METHOD_NOT_ALLOWED,
            "aud_override flag is not supported for now",
        )
    })?;
    //let jwk = RSA_JWK::new_256_aqab(input_conversion::google_pk_kid_str, input_conversion::google_pk_mod_str);

    let (circuit_input_signals, public_inputs_hash) =
        derive_circuit_input_signals(input, &config, None).map_err(|e| {
            error::make_error(
                e,
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting input to circuit-friendly version",
            )
        })?;
    let formatted_input_str = serde_json::to_string(&circuit_input_signals.to_json_value())
        .map_err(|_e| {
            error::make_error(
                anyhow!(""),
                StatusCode::INTERNAL_SERVER_ERROR,
                "error converting input to circuit-friendly version",
            )
        })?;

    // For debugging:
    fs::write("formatted_input.json", &formatted_input_str).unwrap();

    witness_gen(&formatted_input_str)
            .map_err(|e| {
                error::make_error(
                    e,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "problem with witness gen",
                )
            })?;

    // TODO fix this ugly mess
    let (json, internal_metrics) = {
        let full_prover = &mut state_unlocked.full_prover;
        let (json, internal_metrics) = full_prover
            .prove(&formatted_input_str)
            .map_err(error::handle_prover_lib_error)?;
        let json = String::from(json);
        (json, internal_metrics)
    };
    let proof = encode_proof(
        &Value::from_str(&json)
            .map_err(anyhow::Error::from)
            .map_err(|e| {
                error::make_error(
                    e,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Rapidsnark c++ library returned malformed json",
                )
            })?,
    );

    let span = info_span!(
        "Proof generation finished, building response",
        rapidsnark_response_json = json
    );
    let _enter = span.enter();

    let message_to_sign: Groth16ZkpAndStatement = Groth16ZkpAndStatement {
        proof,
        public_inputs_hash: PoseidonHash::from_fr(&public_inputs_hash),
    };

    let training_wheels_signature = state_unlocked
        .private_key
        .sign(&message_to_sign)
        .map_err(anyhow::Error::from)
        .map_err(|e| {
            error::make_error(
                e,
                StatusCode::BAD_REQUEST,
                "Input is invalid or malformatted",
            )
        })?;

    let response = ProverServerResponse::Success {
        proof,
        public_inputs_hash: PoseidonHash::from_fr(&public_inputs_hash),
        training_wheels_signature,
    };

    // this is for debugging. Verify the signature to double-check that we are doing things
    // correctly
    println!(
        "{:?}",
        verify_response_signature(&response, &state_unlocked.public_key)
    );

    state_unlocked
        .metrics
        .witness_generation_time
        .observe((f64::from(internal_metrics.witness_generation_time)) / 1000.0);
    state_unlocked
        .metrics
        .groth16_time
        .observe((f64::from(internal_metrics.prover_time)) / 1000.0);
    state_unlocked
        .metrics
        .response_time
        .observe(start_time.elapsed().as_secs_f64());

    Ok(Json(response))
}

pub async fn metrics_handler(State(state): State<Arc<Mutex<ProverServerState>>>) -> String {
    let state_unlocked = state.lock().expect("Can't lock mutex");
    state_unlocked.metrics.encode_as_string()
}

pub async fn healthcheck_handler() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}

pub async fn fallback_handler() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Incorrect route.")
}

// For debugging.
fn verify_response_signature(
    response: &ProverServerResponse,
    pub_key: &Ed25519PublicKey,
) -> Result<(), anyhow::Error> {
    match response {
        ProverServerResponse::Error {..} => panic!("Should never call verify_response_signature on a response of type ProverServerResponse::Error"),
        ProverServerResponse::Success {  proof, public_inputs_hash, training_wheels_signature } => training_wheels_signature.verify(
            &Groth16ZkpAndStatement {
                proof: *proof,
                public_inputs_hash: *public_inputs_hash
            }
            , pub_key)
    }
}

fn val_to_str_vec(a: &Value) -> [&str; 2] {
    let a: Vec<&str> = a
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    [a[0], a[1]]
}

pub fn encode_proof(proof: &Value) -> Groth16Zkp {
    let pi_a = proof.get("pi_a").unwrap();
    let pi_a_array = pi_a.as_array().unwrap();
    let new_pi_a = G1Bytes::new_unchecked(
        pi_a_array[0].as_str().unwrap(),
        pi_a_array[1].as_str().unwrap(),
    )
    .unwrap();

    let pi_b = proof.get("pi_b").unwrap();
    let pi_b_array = pi_b.as_array().unwrap();
    let new_pi_b = G2Bytes::new_unchecked(
        val_to_str_vec(&pi_b_array[0]),
        val_to_str_vec(&pi_b_array[1]),
    )
    .unwrap();

    let pi_c = proof.get("pi_c").unwrap();
    let pi_c_array = pi_c.as_array().unwrap();
    let new_pi_c = G1Bytes::new_unchecked(
        pi_c_array[0].as_str().unwrap(),
        pi_c_array[1].as_str().unwrap(),
    )
    .unwrap();

    Groth16Zkp::new(new_pi_a, new_pi_b, new_pi_c)
}
