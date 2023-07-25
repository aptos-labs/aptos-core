// Copyright © Aptos Foundation

use once_cell::sync::Lazy;
use prometheus::{register_histogram_vec, Histogram, HistogramVec};

pub static API_TEST_SUCCESS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_success",
        "Number of user flows which succesfully passed",
        &["test_name", "network_name"],
    )
    .unwrap()
});

pub fn test_success(test_name: &str, network_name: &str) -> Histogram {
    API_TEST_SUCCESS.with_label_values(&[test_name, network_name])
}

pub static API_TEST_FAIL: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_fail",
        "Number of user flows which failed checks",
        &["test_name", "network_name"],
    )
    .unwrap()
});

pub fn test_fail(test_name: &str, network_name: &str) -> Histogram {
    API_TEST_FAIL.with_label_values(&[test_name, network_name])
}

pub static API_TEST_ERROR: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!("api_test_error", "Number of user flows which crashed", &[
        "test_name",
        "network_name",
    ])
    .unwrap()
});

pub fn test_error(test_name: &str, network_name: &str) -> Histogram {
    API_TEST_ERROR.with_label_values(&[test_name, network_name])
}

pub static API_TEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "api_test_latency",
        "Time it takes to complete a user flow",
        &["test_name", "network_name", "result"],
    )
    .unwrap()
});

pub fn test_latency(test_name: &str, network_name: &str, result: &str) -> Histogram {
    API_TEST_LATENCY.with_label_values(&[test_name, network_name, result])
}
