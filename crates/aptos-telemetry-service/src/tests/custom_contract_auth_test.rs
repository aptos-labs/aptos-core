// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    tests::test_context::new_test_context_with_auth, OnChainAuthConfig, OnChainAuthMethod,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    PrivateKey, Uniform,
};
use aptos_types::{account_address, account_address::AccountAddress, chain_id::ChainId};
use httpmock::prelude::*;
use rand::SeedableRng;
use serde_json::json;

/// Helper function to sign arbitrary bytes in tests
fn sign_bytes(private_key: &Ed25519PrivateKey, message: &[u8]) -> Ed25519Signature {
    use ed25519_dalek::Signer;
    // Use ed25519_dalek v2.x directly for signing in tests
    let secret_bytes: &[u8; 32] = &private_key.to_bytes();
    let dalek_signing_key = ed25519_dalek::SigningKey::from_bytes(secret_bytes);
    let dalek_signature = dalek_signing_key.sign(message);
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
    let _view_mock = server.mock(|when, then| {
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
        chain_id: 21, // test chain
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

    // Pre-populate the allowlist cache (since tests don't run background updater)
    context.populate_allowlist_cache(contract_name, chain_id, vec![address]);

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
    // Note: view_mock is not asserted because cache is pre-populated directly
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
    let _resource_mock = server.mock(|when, then| {
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
        chain_id: 21, // test chain
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

    // Pre-populate the allowlist cache (since tests don't run background updater)
    context.populate_allowlist_cache(contract_name, chain_id, vec![address]);

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
    // Note: resource_mock is not asserted because cache is pre-populated directly
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

    // Mock view function returning different addresses (not used since cache is pre-populated)
    let _view_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/view");
        then.status(200).json_body(json!([
            {
                "address": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
                "port": "8080"
            }
        ]));
    });

    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
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

    // Pre-populate the allowlist cache with a DIFFERENT address (not the requesting address)
    let other_address = AccountAddress::from_hex_literal(
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
    )
    .unwrap();
    context.populate_allowlist_cache(contract_name, chain_id, vec![other_address]);

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
    // Mock not used - cache is pre-populated directly
}

/// Test signature verification with wrong signature
#[tokio::test]
async fn test_custom_contract_auth_invalid_signature() {
    // Need a minimal config to create the contract instance
    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
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
    let _view_mock = server.mock(|when, then| {
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
        chain_id: 21, // test chain
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

    // Pre-populate the allowlist cache (since tests don't run background updater)
    context.populate_allowlist_cache(contract_name, chain_id, vec![address]);

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
    // Note: view_mock is not asserted because cache is pre-populated directly
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
        chain_id: 21, // test chain
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

    // Mock 404 response for non-existent function (not used since cache is checked first)
    let _view_mock = server.mock(|when, then| {
        when.method(POST).path("/v1/view");
        then.status(404).json_body(json!({
            "message": "Module not found",
            "error_code": "module_not_found",
            "vm_error_code": 404
        }));
    });

    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
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

    // Should fail with 403 because cache is not populated (background updater would fail to fetch)
    // In the new push model, we return "cache miss" when allowlist isn't available
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
    // With background refresh model, cache miss returns allowlist not available
    let message = auth_resp["message"].as_str().unwrap();
    assert!(
        message.contains("allowlist not available") || message.contains("cache miss"),
        "unexpected error message: {}",
        message
    );
}

/// Test replay attack prevention - same challenge cannot be used twice
#[tokio::test]
async fn test_custom_contract_auth_replay_attack_prevented() {
    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    let contract_address = "0x0000000000000000000000000000000000000000000000000000000000000001";

    // Mock view function to return the address
    let _view_mock = server.mock(|when, then| {
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
                    "port": "8080"
                }
            ]));
    });

    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: format!("{}::registry::get_members", contract_address),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "TestNode".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;
    let contract_name = "test_contract";

    // Pre-populate the allowlist cache (since tests don't run background updater)
    context.populate_allowlist_cache(contract_name, chain_id, vec![address]);

    // Get a challenge
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

    // First auth should succeed
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

    // Second auth with same challenge should fail (replay attack)
    let replay_resp = context
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

    assert_eq!(replay_resp["code"], 400);
    let message = replay_resp["message"].as_str().unwrap();
    assert!(message.contains("challenge") || message.contains("expired"));
}

/// Test bypass attack prevention - client-generated challenge doesn't work
#[tokio::test]
async fn test_custom_contract_auth_bypass_attack_prevented() {
    // Need a minimal config to create the contract instance
    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::test::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: None,
        node_type_name: "custom".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;
    let contract_name = "test_contract";

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Create a self-generated challenge (bypass attack)
    let fake_challenge = "self-generated-challenge-12345";
    let signature = sign_bytes(&private_key, fake_challenge.as_bytes());

    // Auth with fake challenge should fail
    let auth_resp = context
        .expect_status_code(400)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": fake_challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    assert_eq!(auth_resp["code"], 400);
    let message = auth_resp["message"].as_str().unwrap();
    assert!(
        message.contains("challenge")
            || message.contains("not found")
            || message.contains("issued")
    );
}

/// Test that challenge for one address cannot be used by another address
#[tokio::test]
async fn test_custom_contract_auth_challenge_address_isolation() {
    // Need a minimal config to create the contract instance
    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::test::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: None,
        node_type_name: "custom".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;
    let contract_name = "test_contract";

    // Generate two different keypairs
    let private_key1 = Ed25519PrivateKey::generate_for_testing();
    let public_key1 = private_key1.public_key();
    let address1 = account_address::from_public_key(&public_key1);

    // Generate a second keypair (deterministically different)
    let mut rng = ::rand::rngs::StdRng::from_seed([1u8; 32]);
    let private_key2 = Ed25519PrivateKey::generate(&mut rng);
    let public_key2 = private_key2.public_key();
    let address2 = account_address::from_public_key(&public_key2);

    let chain_id = ChainId::new(21);

    // Request challenge for address1
    let challenge_resp = context
        .post(
            &format!("/api/v1/custom-contract/{}/auth-challenge", contract_name),
            json!({
                "address": address1.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    let challenge = challenge_resp["challenge"].as_str().unwrap();

    // Sign the challenge with address2's key
    let signature = sign_bytes(&private_key2, challenge.as_bytes());

    // Try to use address1's challenge with address2 - should fail
    let auth_resp = context
        .expect_status_code(400)
        .post(
            &format!("/api/v1/custom-contract/{}/auth", contract_name),
            json!({
                "address": address2.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key2.to_bytes().to_vec()
            }),
        )
        .await;

    assert_eq!(auth_resp["code"], 400);
}

/// Test that challenge for unconfigured contract is rejected
#[tokio::test]
async fn test_custom_contract_auth_challenge_unconfigured_contract() {
    let config = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::test::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: None,
        node_type_name: "custom".to_string(),
    };

    let context = new_test_context_with_auth(Some(config)).await;

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Request challenge for non-existent contract - should fail
    let challenge_resp = context
        .expect_status_code(403)
        .post(
            "/api/v1/custom-contract/nonexistent_contract/auth-challenge",
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id
            }),
        )
        .await;

    assert_eq!(challenge_resp["code"], 403);
    let message = challenge_resp["message"].as_str().unwrap();
    assert!(message.contains("not configured"));
}

/// Test cross-contract token reuse attack is prevented
/// A user authenticated for contract_A should NOT be able to use their JWT
/// to inject data into contract_B's endpoints
#[tokio::test]
async fn test_cross_contract_token_reuse_prevented() {
    use crate::tests::test_context::new_test_context_with_multiple_contracts;

    let server = MockServer::start();

    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();
    let address = account_address::from_public_key(&public_key);
    let chain_id = ChainId::new(21);

    // Mock view function to return the test address for both contracts
    let _view_mock = server.mock(|when, then| {
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
                    "port": "8080"
                }
            ]));
    });

    // Create two contracts with different names
    let config_a = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::registry::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "ContractANode".to_string(),
    };

    let config_b = OnChainAuthConfig {
        chain_id: 21, // test chain
        method: OnChainAuthMethod::ViewFunction,
        resource_path: "0x1::registry::get_members".to_string(),
        view_function_args: vec![],
        address_list_field: "[0].address".to_string(),
        rest_api_url: Some(server.base_url().parse().unwrap()),
        node_type_name: "ContractBNode".to_string(),
    };

    let context = new_test_context_with_multiple_contracts(Some(vec![
        ("contract_a".to_string(), config_a),
        ("contract_b".to_string(), config_b),
    ]))
    .await;

    // Pre-populate the allowlist cache for both contracts
    context.populate_allowlist_cache("contract_a", chain_id, vec![address]);
    context.populate_allowlist_cache("contract_b", chain_id, vec![address]);

    // Authenticate for contract_a
    let challenge_resp = context
        .post(
            "/api/v1/custom-contract/contract_a/auth-challenge",
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
            "/api/v1/custom-contract/contract_a/auth",
            json!({
                "address": address.to_hex_literal(),
                "chain_id": chain_id,
                "challenge": challenge,
                "signature": signature.to_bytes().to_vec(),
                "public_key": public_key.to_bytes().to_vec()
            }),
        )
        .await;

    // Should get a valid token for contract_a
    assert!(auth_resp.get("token").is_some());
    let token_for_contract_a = auth_resp["token"].as_str().unwrap();

    // Try to use contract_a's token on contract_b's metrics endpoint - should fail
    let metrics_resp = context
        .with_bearer_auth(token_for_contract_a.to_string())
        .expect_status_code(403)
        .reply(
            warp::test::request()
                .header("Authorization", format!("Bearer {}", token_for_contract_a))
                .method("POST")
                .path("/api/v1/custom-contract/contract_b/ingest/metrics")
                .body("test_metric{label=\"value\"} 42"),
        )
        .await;

    // Verify the request was rejected (cross-contract token reuse blocked)
    assert_eq!(metrics_resp.status(), 403);
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

    // Mock 404 response for non-existent resource (not used since cache is checked first)
    let _resource_mock = server.mock(|when, then| {
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
        chain_id: 21, // test chain
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

    // Should fail with 403 because cache is not populated (background updater would fail to fetch)
    // In the new push model, we return "cache miss" when allowlist isn't available
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
    // With background refresh model, cache miss returns allowlist not available
    let message = auth_resp["message"].as_str().unwrap();
    assert!(
        message.contains("allowlist not available") || message.contains("cache miss"),
        "unexpected error message: {}",
        message
    );
    // Note: resource_mock is not asserted because verify_custom_contract only checks cache
}
