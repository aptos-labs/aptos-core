// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cached_resources::CachedResources,
    groth16_vk::OnChainGroth16VerificationKey,
    keyless_config::OnChainKeylessConfiguration,
    request_handler::{
        handle_request, ABOUT_PATH, DEFAULT_PEPPER_SERVICE_PORT, FETCH_PATH, GROTH16_VK_PATH,
        KEYLESS_CONFIG_PATH, SIGNATURE_PATH, VERIFY_PATH, VUF_PUB_KEY_PATH,
    },
    tests::utils,
};
use hyper::{
    header::{
        ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    },
    Body, Method, Request, Response, StatusCode,
};
use reqwest::header::ACCESS_CONTROL_ALLOW_CREDENTIALS;
use std::{ops::Deref, sync::Arc};

#[tokio::test]
async fn test_options_request() {
    // Send an options request to the root path
    let response = send_request_to_path(Method::OPTIONS, "/", Body::empty(), None, None).await;

    // Assert that the response status is OK
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the response headers
    let headers = response.headers();
    assert_eq!(headers.get(ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "");
    assert_eq!(
        headers.get(ACCESS_CONTROL_ALLOW_CREDENTIALS).unwrap(),
        "true"
    );
    assert_eq!(headers.get(ACCESS_CONTROL_ALLOW_HEADERS).unwrap(), "*");
    assert_eq!(
        headers.get(ACCESS_CONTROL_ALLOW_METHODS).unwrap(),
        "GET, POST, OPTIONS"
    );
}

#[tokio::test]
async fn test_get_about_request() {
    // Send a GET request to the about endpoint
    let response = send_request_to_path(Method::GET, ABOUT_PATH, Body::empty(), None, None).await;

    // Assert that the response status is OK
    assert_eq!(response.status(), StatusCode::OK);

    // Parse the response body as a JSON map
    let body_string = get_response_body_string(response).await;
    let json_value: serde_json::Value = serde_json::from_str(&body_string).unwrap();
    let json_map = json_value.as_object().unwrap();

    // Verify the response body contains relevant build information
    assert!(json_map.contains_key("build_cargo_version"));
    assert!(json_map.contains_key("build_commit_hash"));
    assert!(json_map.contains_key("build_is_release_build"));
}

#[tokio::test]
async fn test_get_keyless_config_request() {
    // Create a cached resources object with no cached resources
    let cached_resources = CachedResources::default();

    // Send a GET request to the keyless config endpoint
    let response = send_request_to_path(
        Method::GET,
        KEYLESS_CONFIG_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 404 (the resource has not been cached yet)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Update the keyless config cached resource
    let mut on_chain_keyless_configuration = OnChainKeylessConfiguration::default();
    cached_resources.set_on_chain_keyless_configuration(on_chain_keyless_configuration.clone());

    // Send a GET request to the keyless config endpoint
    let response = send_request_to_path(
        Method::GET,
        KEYLESS_CONFIG_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 200 (the resource has been cached)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the keyless config from the response body JSON
    let body_string = get_response_body_string(response).await;
    let response_keyless_config: OnChainKeylessConfiguration =
        serde_json::from_str(&body_string).unwrap();
    assert_eq!(response_keyless_config, on_chain_keyless_configuration);

    // Update the keyless config cached resource with a new config
    on_chain_keyless_configuration.data.max_exp_horizon_secs = "Some new value".into();
    cached_resources.set_on_chain_keyless_configuration(on_chain_keyless_configuration.clone());

    // Send a GET request to the keyless config endpoint
    let response = send_request_to_path(
        Method::GET,
        KEYLESS_CONFIG_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 200 (the new resource has been cached)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the new keyless config from the response body
    let body_string = get_response_body_string(response).await;
    let response_keyless_config: OnChainKeylessConfiguration =
        serde_json::from_str(&body_string).unwrap();
    assert_eq!(response_keyless_config, on_chain_keyless_configuration);
}

#[tokio::test]
async fn test_get_groth16_vk_request() {
    // Create a cached resources object with no cached resources
    let cached_resources = CachedResources::default();

    // Send a GET request to the groth16 vk endpoint
    let response = send_request_to_path(
        Method::GET,
        GROTH16_VK_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 404 (the resource has not been cached yet)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Update the groth16 vk cached resource
    let mut on_chain_groth16_vk = OnChainGroth16VerificationKey::default();
    cached_resources.set_on_chain_groth16_vk(on_chain_groth16_vk.clone());

    // Send a GET request to the groth16 vk endpoint
    let response = send_request_to_path(
        Method::GET,
        GROTH16_VK_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 200 (the resource has been cached)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the groth16 vk from the response body JSON
    let body_string = get_response_body_string(response).await;
    let response_groth16_vk: OnChainGroth16VerificationKey =
        serde_json::from_str(&body_string).unwrap();
    assert_eq!(response_groth16_vk, on_chain_groth16_vk);

    // Update the groth16 vk cached resource with a new
    on_chain_groth16_vk.data.beta_g2 = "Some new value".into();
    cached_resources.set_on_chain_groth16_vk(on_chain_groth16_vk.clone());

    // Send a GET request to the groth16 vk endpoint
    let response = send_request_to_path(
        Method::GET,
        GROTH16_VK_PATH,
        Body::empty(),
        None,
        Some(cached_resources.clone()),
    )
    .await;

    // Assert that the response is a 200 (the new resource has been cached)
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the new groth16 vk from the response body
    let body_string = get_response_body_string(response).await;
    let response_groth16_vk: OnChainGroth16VerificationKey =
        serde_json::from_str(&body_string).unwrap();
    assert_eq!(response_groth16_vk, on_chain_groth16_vk);
}

#[tokio::test]
async fn test_get_vuf_pub_key_request() {
    // Generate a test VUF public private keypair
    let vuf_keypair = utils::create_vuf_public_private_keypair();

    // Send a GET request to the vuf public key endpoint
    let response = send_request_to_path(
        Method::GET,
        VUF_PUB_KEY_PATH,
        Body::empty(),
        Some(vuf_keypair.clone()),
        None,
    )
    .await;

    // Assert that the response is a 200 (OK)
    assert_eq!(response.status(), StatusCode::OK);

    // Get the public key from the response body
    let body_string = get_response_body_string(response).await;
    let response_vuf_public_key = get_public_key_from_json(&body_string);

    // Get the expected public key from the keypair
    let (vuf_public_key_json, _) = vuf_keypair.deref();
    let vuf_public_key = get_public_key_from_json(vuf_public_key_json);

    // Verify the public key is correct
    assert_eq!(response_vuf_public_key, vuf_public_key);
}

#[tokio::test]
async fn test_get_invalid_path_or_method_request() {
    // Send a GET request to an unknown endpoint and verify that it returns 404
    let response =
        send_request_to_path(Method::GET, "/invalid_path", Body::empty(), None, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Send a GET request to an endpoint that only supports POST requests, and verify that it returns 405
    let response = send_request_to_path(Method::GET, VERIFY_PATH, Body::empty(), None, None).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    // Send a POST request to an endpoint that only supports GET requests, and verify that it returns 405
    let response = send_request_to_path(Method::POST, ABOUT_PATH, Body::empty(), None, None).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// TODO: add tests that check the fetch logic
#[tokio::test]
async fn test_post_fetch_request_bad_request() {
    // Send a POST request to the fetch endpoint
    let response = send_request_to_path(Method::POST, FETCH_PATH, Body::empty(), None, None).await;

    // Assert that the response is a 400 (bad request, since no body was provided)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// TODO: add tests that check the signature logic
#[tokio::test]
async fn test_post_signature_request_bad_request() {
    // Send a POST request to the signature endpoint
    let response =
        send_request_to_path(Method::POST, SIGNATURE_PATH, Body::empty(), None, None).await;

    // Assert that the response is a 400 (bad request, since no body was provided)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// TODO: add tests that check the verify logic
#[tokio::test]
async fn test_post_verify_request_bad_request() {
    // Send a POST request to the verify endpoint
    let response = send_request_to_path(Method::POST, VERIFY_PATH, Body::empty(), None, None).await;

    // Assert that the response is a 400 (bad request, since no body was provided)
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

/// Gets the public key from a JSON string map
fn get_public_key_from_json(json_string_map: &str) -> String {
    let json_value: serde_json::Value = serde_json::from_str(json_string_map).unwrap();
    let json_map = json_value.as_object().unwrap();
    let json_entry = json_map.get("public_key").unwrap();
    json_entry.as_str().unwrap().to_string()
}

/// Gets the response body as a string
async fn get_response_body_string(response: Response<Body>) -> String {
    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    String::from_utf8(body_bytes.to_vec()).unwrap()
}

// Calls the request handler with the given method, endpoint, and body
async fn send_request_to_path(
    method: Method,
    endpoint: &str,
    body: Body,
    vuf_keypair: Option<Arc<(String, ark_bls12_381::Fr)>>,
    cached_resources: Option<CachedResources>,
) -> Response<Body> {
    // Build the URI
    let uri = format!(
        "http://127.0.0.1:{}{}",
        DEFAULT_PEPPER_SERVICE_PORT, endpoint
    );

    // Build the request
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .body(body)
        .unwrap();

    // Get or create a VUF public private keypair
    let vuf_keypair = vuf_keypair.unwrap_or_else(utils::create_vuf_public_private_keypair);

    // Get or create cached resources
    let cached_resources = cached_resources.unwrap_or_default();

    // Serve the request
    handle_request(request, vuf_keypair, cached_resources)
        .await
        .unwrap()
}
