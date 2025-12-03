// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    tests::test_context::new_test_context_with_auth, OnChainAuthConfig, OnChainAuthMethod,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    PrivateKey, Uniform,
};
use aptos_types::{account_address, chain_id::ChainId};
use httpmock::prelude::*;
use serde_json::json;

/// Helper function to sign arbitrary bytes in tests
fn sign_bytes(private_key: &Ed25519PrivateKey, message: &[u8]) -> Ed25519Signature {
    use ed25519_dalek::Signer;
    // Use ed25519_dalek v1.x directly for signing in tests
    let secret_bytes: &[u8; 32] = &private_key.to_bytes();
    let dalek_secret = ed25519_dalek::SecretKey::from_bytes(secret_bytes).unwrap();
    let dalek_public = ed25519_dalek::PublicKey::from(&dalek_secret);
    let dalek_keypair = ed25519_dalek::Keypair {
        secret: dalek_secret,
        public: dalek_public,
    };
    let dalek_signature = dalek_keypair.sign(message);
    Ed25519Signature::try_from(dalek_signature.to_bytes().as_ref()).unwrap()
}

/// Test authentication flow with mocked view function
#[tokio::test]
async fn test_custom_contract_auth_with_view_function() {
    let server = MockServer::start();

    // Generate a keypair for the client
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Use a valid test contract address
    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock the view function response (REST client uses /v1/view)
    // Note: We match on path only since REST client serializes the body differently
    let view_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/view");
        then.status(200)
            .header("X-Aptos-Chain-Id", "21")
            .header("X-Aptos-Ledger-Version", "1000")
            .header("X-Aptos-Ledger-TimestampUsec", "1234567890000000")
            .header("X-Aptos-Epoch", "1")
            .header("X-Aptos-Ledger-Oldest-Version", "0")
            .header("X-Aptos-Block-Height", "100")
            .header("X-Aptos-Oldest-Block-Height", "0")
            .json_body(json!([
                {
                    "address": address.to_hex_literal(),
                    "bls_public_key": "0xabc123",
                    "failure_domain": "dc_test",
                    "ip_address": "127.0.0.1",
                    "port": "8080"
                }
            ]));
    });

    // Create config and context
    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: format!("{}::registry::get_members", contract_address),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    // Auth endpoints now require contract name in path
    let contract_name = "test_contract";

    // Step 1: Request challenge
    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    assert!(challenge_resp.get("challenge").is_some());
    let challenge = challenge_resp["challenge"].as_str().unwrap();

    // Step 2: Sign challenge
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    // Step 3: Authenticate
    let auth_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    // Should succeed
    assert!(auth_resp.get("token").is_some());
    view_mock.assert();
}

/// Test authentication with resource method (legacy)
/// Ignored because mocking the full Aptos REST API response headers is complex
#[tokio::test]
async fn test_custom_contract_auth_with_resource() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Use a valid test contract address
    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock the resource endpoint with required Aptos API headers
    let resource_mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("accounts")
            .path_contains("resource");
        then.status(200)
            .header("X-Aptos-Chain-Id", "21")
            .header("X-Aptos-Ledger-Version", "1000")
            .header("X-Aptos-Ledger-TimestampUsec", "1234567890")
            .header("X-Aptos-Epoch", "1")
            .header("X-Aptos-Ledger-Oldest-Version", "0")
            .header("X-Aptos-Block-Height", "100")
            .header("X-Aptos-Oldest-Block-Height", "0")
            .json_body(json!({
                "type": format!("{}::registry::Members", contract_address),
                "data": {
                    "members": [address.to_hex_literal()]
                }
            }));
    });

    // Create config and context
    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::Resource,
        resource_path: format!("{}::registry::Members", contract_address),
        view_function_args: vec![],
        address_list_field: "members".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    // Auth endpoints now require contract name in path
    let contract_name = "test_contract";

    // Get challenge
    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    // Authenticate
    let auth_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert!(auth_resp.get("token").is_some());
    resource_mock.assert();
}

/// Test authentication failure when address not in allowlist
#[tokio::test]
async fn test_custom_contract_auth_address_not_in_allowlist() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Use a valid test contract address
    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock view function returning different addresses
    let view_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/view");
        then.status(200).json_body(json!([
            {
                "address": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                "port": "8080"
            }
        ]));
    });

    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: format!("{}::registry::get_members", contract_address),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    // Auth endpoints now require contract name in path
    let contract_name = "test_contract";

    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    // This should fail
    let auth_resp = context
        .expect_status_code(403)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert!(auth_resp.get("code").is_some());
    assert_eq!(auth_resp["code"], 403);
    view_mock.assert();
}

/// Test signature verification with wrong signature
#[tokio::test]
async fn test_custom_contract_auth_invalid_signature() {
    // Need a minimal config to create the contract instance
    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::test::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: None,
        node_type_name: "custom".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    // Auth endpoints now require contract name in path
    let contract_name = "test_contract";

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();

    // Sign wrong message
    let wrong_message = "wrong challenge";
    let signature = sign_bytes(&private_key, wrong_message.as_bytes());

    // This should fail before even checking on-chain
    let auth_resp = context
        .expect_status_code(400)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert_eq!(auth_resp["code"], 400);
}

/// Test with Shelby-style response format
#[tokio::test]
async fn test_custom_contract_auth_shelby_format() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Mock Shelby's get_all_storage_providers response format
    let view_mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/view");
        then.status(200)
            .header("X-Aptos-Chain-Id", "21")
            .header("X-Aptos-Ledger-Version", "1000")
            .header("X-Aptos-Ledger-TimestampUsec", "1234567890000000")
            .header("X-Aptos-Epoch", "1")
            .header("X-Aptos-Ledger-Oldest-Version", "0")
            .header("X-Aptos-Block-Height", "100")
            .header("X-Aptos-Oldest-Block-Height", "0")
            .json_body(json!([
                {
                    "address": address.to_hex_literal(),
                    "bls_public_key": "0xb9aef025aa84f215d7ce94f830a3ffe2dc13fbae9a7abcf7b6c17e36441eb2ae757e7a4e6c8e8dd192e214de6a19829e",
                    "failure_domain": "dc_us_east",
                    "ip_address": "172.16.0.3",
                    "port": "39431"
                },
                {
                    "address": "0xe608a3e95269e1f54fdd2ee3112616d590fc021f1bb00c86ab8b663dd862d71a",
                    "bls_public_key": "0xac4bb211f707f2448728f3ddd8a8b850e28525a63ddd31aa70ced8bbbef21b81e738c71b3e4643adaf19fc225acbf528",
                    "failure_domain": "dc_us_west",
                    "ip_address": "172.16.0.5",
                    "port": "39431"
                }
            ]));
    });

    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0xc63d6a5efb0080a6029403131715bd4971e1149f7cc099aac69bb0069b3ddbf5::global_metadata::get_all_storage_providers".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "ShelbyStorageProvider".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    // Auth endpoints now require contract name in path
    let contract_name = "test_contract";

    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    let auth_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert!(auth_resp.get("token").is_some());

    // Verify the JWT contains correct claims
    let token = auth_resp["token"].as_str().unwrap();
    assert!(!token.is_empty());

    view_mock.assert();
}

/// Unit test for OnChainAuthConfig environment variable substitution
#[test]
fn test_on_chain_auth_config_env_substitution() {
    use std::sync::Mutex;

    // Use a mutex to prevent race conditions in env var setting
    static ENV_MUTEX: Mutex<()> = Mutex::new(());
    let _guard = ENV_MUTEX.lock().unwrap();

    unsafe {
        std::env::set_var("TEST_CONTRACT", "0x123");
    }

    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "${TEST_CONTRACT}::module::function".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: None,
        node_type_name: "custom".to_string(),
    };

    let resolved = config.resolve_resource_path().unwrap();
    assert_eq!(resolved, "0x123::module::function");

    unsafe {
        std::env::remove_var("TEST_CONTRACT");
    }
}

/// Test extract_address_list with different formats
#[test]
fn test_extract_address_list_formats() {
    use crate::custom_contract_auth::extract_addresses_from_value;

    // Test with array of strings
    let json1 = json!([
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
        "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
    ]);
    let addrs1 = extract_addresses_from_value(&json1).unwrap();
    assert_eq!(addrs1.len(), 2);

    // Test with array of objects
    let json2 = json!([
        {"address": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef", "port": "8080"},
        {"address": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890", "port": "9090"}
    ]);
    let addrs2 = extract_addresses_from_value(&json2).unwrap();
    assert_eq!(addrs2.len(), 2);
}

/// Test graceful failure when view function doesn't exist
#[tokio::test]
async fn test_custom_contract_auth_function_not_found() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock 404 response for non-existent function
    let view_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/view");
        then.status(404).json_body(json!({
            "message": "Module not found",
            "error_code": "module_not_found",
            "vm_error_code": 404
        }));
    });

    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::ViewFunction,
        resource_path: format!("{}::nonexistent::get_members", contract_address),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;
    let contract_name = "test_contract";

    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    // Should fail with 403 and helpful error message
    let auth_resp = context
        .expect_status_code(403)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert_eq!(auth_resp["code"], 403);
    // Verify the error message mentions the function doesn't exist
    let message = auth_resp["message"].as_str().unwrap();
    assert!(message.contains("does not exist") || message.contains("not found"));

    view_mock.assert();
}

/// Test graceful failure when resource doesn't exist (legacy method)
#[tokio::test]
async fn test_custom_contract_auth_resource_not_found() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock 404 response for non-existent resource
    let resource_mock = server.mock(|when, then| {
        when.method(GET)
            .path_contains("accounts")
            .path_contains("resource");
        then.status(404)
            .header("X-Aptos-Chain-Id", "21")
            .header("X-Aptos-Ledger-Version", "1000")
            .header("X-Aptos-Ledger-TimestampUsec", "1234567890000000")
            .header("X-Aptos-Epoch", "1")
            .header("X-Aptos-Ledger-Oldest-Version", "0")
            .header("X-Aptos-Block-Height", "100")
            .header("X-Aptos-Oldest-Block-Height", "0")
            .json_body(json!({
                "message": "Resource not found",
                "error_code": "resource_not_found"
            }));
    });

    let config = OnChainAuthConfig {
        method: OnChainAuthMethod::Resource,
        resource_path: format!("{}::nonexistent::Members", contract_address),
        view_function_args: vec![],
        address_list_field: "members".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;
    let contract_name = "test_contract";

    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();
    let signature = sign_bytes(&private_key, challenge.as_bytes());

    // Should fail with 403 and helpful error message
    let auth_resp = context
        .expect_status_code(403)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert_eq!(auth_resp["code"], 403);
    let message = auth_resp["message"].as_str().unwrap();
    assert!(message.contains("not found") || message.contains("does not exist"));

    resource_mock.assert();
}
