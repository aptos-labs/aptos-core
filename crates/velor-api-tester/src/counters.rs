// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use prometheus::{register_histogram_vec, Histogram, HistogramVec};

pub static API_TEST_SUCCESS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_success",
        "Number of user flows which succesfully passed",
        &["test_name", "network_name", "run_id"],
    )
    .unwrap()
});

pub fn test_success(test_name: &str, network_name: &str, run_id: &str) -> Histogram {
    API_TEST_SUCCESS.with_label_values(&[test_name, network_name, run_id])
}

pub static API_TEST_FAIL: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_fail",
        "Number of user flows which failed checks",
        &["test_name", "network_name", "run_id"],
    )
    .unwrap()
});

pub fn test_fail(test_name: &str, network_name: &str, run_id: &str) -> Histogram {
    API_TEST_FAIL.with_label_values(&[test_name, network_name, run_id])
}

pub static API_TEST_ERROR: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!("api_test_error", "Number of user flows which crashed", &[
        "test_name",
        "network_name",
        "run_id"
    ],)
    .unwrap()
});

pub fn test_error(test_name: &str, network_name: &str, run_id: &str) -> Histogram {
    API_TEST_ERROR.with_label_values(&[test_name, network_name, run_id])
}

pub static API_TEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_latency",
        "Time it takes to complete a user flow",
        &["test_name", "network_name", "run_id", "result"],
    )
    .unwrap()
});

pub fn test_latency(test_name: &str, network_name: &str, run_id: &str, result: &str) -> Histogram {
    API_TEST_LATENCY.with_label_values(&[test_name, network_name, run_id, result])
}

pub static API_TEST_STEP_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_step_latency",
        "Time it takes to complete a user flow step",
        &["test_name", "step_name", "network_name", "run_id", "result"],
    )
    .unwrap()
});

pub fn test_step_latency(
    test_name: &str,
    step_name: &str,
    network_name: &str,
    run_id: &str,
    result: &str,
) -> Histogram {
    API_TEST_STEP_LATENCY.with_label_values(&[test_name, step_name, network_name, run_id, result])
}
