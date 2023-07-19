// Copyright © Aptos Foundation

use once_cell::sync::Lazy;
use prometheus::{register_int_counter_vec, IntCounterVec, IntCounter};

pub static API_TEST_SUCCESS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "api_test_success",
        "Number of user flows which succesfully passed",
        &["test_name", "network_name"]
    )
    .unwrap()
});

pub fn test_success(
    test_name: &str,
    network_name: &str,
) -> IntCounter {
    API_TEST_SUCCESS.with_label_values(&[
        test_name,
        network_name,
    ])
}

pub static API_TEST_FAIL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "api_test_fail",
        "Number of user flows which failed checks",
        &["test_name", "network_name"]
    )
    .unwrap()
});

pub fn test_fail(
    test_name: &str,
    network_name: &str,
) -> IntCounter {
    API_TEST_FAIL.with_label_values(&[
        test_name,
        network_name,
    ])
}

pub static API_TEST_ERROR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "api_test_error",
        "Number of user flows which crashed",
        &["test_name", "network_name"]
    )
    .unwrap()
});

pub fn test_error(
    test_name: &str,
    network_name: &str,
) -> IntCounter {
    API_TEST_ERROR.with_label_values(&[
        test_name,
        network_name,
    ])
}
