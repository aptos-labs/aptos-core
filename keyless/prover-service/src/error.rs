// Copyright © Aptos Foundation

use crate::{api::ProverServerResponse, logging};
use axum::{http::StatusCode, Json};
use rust_rapidsnark::ProverError;

pub fn make_error(
    e: anyhow::Error,
    code: StatusCode,
    message: &str,
) -> (StatusCode, Json<ProverServerResponse>) {
    logging::do_tracing(e, code, message);
    (
        code,
        Json(ProverServerResponse::Error {
            message: String::from(message),
        }),
    )
}

pub fn handle_prover_lib_error(e: ProverError) -> (StatusCode, Json<ProverServerResponse>) {
    match e {
        ProverError::ProverNotReady => make_error(e.into(), StatusCode::SERVICE_UNAVAILABLE, "Prover is not ready"),
        ProverError::InvalidInput => make_error(e.into(), StatusCode::BAD_REQUEST, "Input is invalid or malformatted"),
        ProverError::WitnessGenerationBinaryProblem => make_error(e.into(), StatusCode::INTERNAL_SERVER_ERROR, "Problem with the witness generation binary"),
        ProverError::WitnessGenerationInvalidCurve => make_error(e.into(), StatusCode::INTERNAL_SERVER_ERROR, "The generated witness file uses a different curve than bn128, which is currently the only supported curve."),
        ProverError::Unknown(s) => make_error(e.into(), StatusCode::INTERNAL_SERVER_ERROR, format!("Unknown error, passing forward everthing I know: {s}").as_str()),
    }
}
