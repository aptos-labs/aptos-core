// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Characterization of the error table (`src/error.rs`).
//!
//! This pins the exact `(code, retriable, message)` for every one of the 36
//! errors so the error table can never drift.  BC-1 (message typo fixes) and
//! BC-2 (InvalidTransactionUpdate mapping) have landed; the expected values
//! below reflect the corrected behavior.

use crate::error::ApiError;
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode},
    error::{AptosErrorResponse, RestError},
};

/// The golden error table: (code, retriable, message).  Ordered by code.
/// Any change to a message/code/retriable flag must be reflected here.
const EXPECTED: &[(u32, bool, &str)] = &[
    (1, false, "Transaction is pending"),
    (2, false, "Network identifier doesn't match"),
    (3, false, "Chain Id doesn't match"),
    (4, false, "Deserialization failed"),
    (5, false, "Invalid operations for a transfer"),
    (6, false, "Invalid signature type"),
    (7, false, "Invalid max gas fee"),
    (
        8,
        false,
        "Max fee is lower than the estimated cost of the transaction",
    ),
    (9, false, "Invalid gas multiplier"),
    (10, false, "Invalid operations"),
    (11, false, "Payload metadata is missing"),
    (12, false, "Currency is unsupported"),
    (13, false, "Number of signatures is not supported"),
    // BC-1: fixed message (was "...because he's offline").
    (
        14,
        false,
        "This API is unavailable because the node is offline",
    ),
    (15, false, "Transaction failed to parse"),
    (16, true, "Gas estimation failed"),
    (17, false, "Internal error"),
    (18, true, "Account not found"),
    (19, false, "Resource not found"),
    (20, false, "Module not found"),
    (21, false, "Struct field not found"),
    (22, false, "Version not found"),
    (23, false, "Transaction not found"),
    (24, false, "Table item not found"),
    // BC-1: fixed message (was "Block is missing events").
    (25, true, "Block not found"),
    (26, false, "Version pruned"),
    (27, false, "Block pruned"),
    (28, false, "Invalid input"),
    (
        29,
        false,
        "Invalid transaction update.  Can only update gas unit price",
    ),
    (
        30,
        false,
        "Sequence number too old.  Please create a new transaction with an updated sequence number",
    ),
    (31, false, "Transaction submission failed due to VM error"),
    (32, true, "Mempool is full all accounts"),
    // BC-1: fixed typo (was "Faileed to retrieve...").
    (
        33,
        true,
        "Failed to retrieve the coin type information, please retry",
    ),
    (34, false, "StateValue not found."),
    (
        35,
        false,
        "Transaction was rejected by the transaction filter",
    ),
    (36, true, "Rate limited"),
];

#[test]
fn error_table_matches_golden() {
    let mut actual: Vec<(u32, bool, &str)> = ApiError::all()
        .into_iter()
        .map(|err| (err.code(), err.retriable(), err.message()))
        .collect();
    actual.sort_by_key(|(code, _, _)| *code);

    let mut expected = EXPECTED.to_vec();
    expected.sort_by_key(|(code, _, _)| *code);

    assert_eq!(actual, expected);
}

#[test]
fn every_error_is_http_500() {
    for err in ApiError::all() {
        assert_eq!(
            err.status_code(),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}

#[test]
fn error_codes_are_unique_and_cover_1_through_36() {
    let mut codes: Vec<u32> = ApiError::all().into_iter().map(|err| err.code()).collect();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes, (1..=36).collect::<Vec<_>>());
}

#[test]
fn invalid_transaction_update_maps_to_matching_error() {
    // BC-2: the node's InvalidTransactionUpdate must map to Rosetta's
    // InvalidTransactionUpdate (code 29), not InvalidInput (code 28).
    let rest_error = RestError::Api(AptosErrorResponse {
        error: AptosError {
            message: "only gas unit price can change".to_string(),
            error_code: AptosErrorCode::InvalidTransactionUpdate,
            vm_error_code: None,
        },
        state: None,
        status_code: warp::http::StatusCode::BAD_REQUEST,
    });
    let api_error: ApiError = rest_error.into();
    assert!(matches!(api_error, ApiError::InvalidTransactionUpdate(_)));
    assert_eq!(api_error.code(), 29);
}

#[test]
fn into_error_carries_code_message_retriable() {
    for err in ApiError::all() {
        let code = err.code();
        let retriable = err.retriable();
        let message = err.message().to_string();
        let wire = err.into_error();
        assert_eq!(wire.code, code);
        assert_eq!(wire.retriable, retriable);
        assert_eq!(wire.message, message);
        // The bare `all()` variants carry no details.
        assert!(wire.details.is_none());
    }
}
