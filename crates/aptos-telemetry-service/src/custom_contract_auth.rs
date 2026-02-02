// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Custom contract authentication via on-chain allowlist verification.
///
/// Flow: Challenge request → Sign challenge → Verify signature + on-chain allowlist → JWT token
use crate::{
    context::Context,
    debug, error,
    errors::{ServiceError, ServiceErrorCode},
    jwt_auth::create_jwt_token,
    metrics::{record_custom_contract_error, CustomContractEndpoint, CustomContractErrorType},
    types::common::NodeType,
    warn,
};
use anyhow::{anyhow, Result};
use aptos_crypto::{
    ed25519::{Ed25519PublicKey, Ed25519Signature},
    CryptoMaterialError, Signature,
};
use aptos_rest_client::Client as RestClient;
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp::{filters::BoxedFilter, reject, reply, Filter, Rejection, Reply};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeRequest {
    pub address: AccountAddress,
    pub chain_id: ChainId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeResponse {
    pub challenge: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAuthRequest {
    pub address: AccountAddress,
    /// Chain ID
    pub chain_id: ChainId,
    /// The challenge that was signed
    pub challenge: String,
    /// Ed25519 signature over the challenge
    pub signature: Vec<u8>,
    /// Ed25519 public key used for signing (32 bytes)
    pub public_key: Vec<u8>,
}

/// Authentication response with JWT token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAuthResponse {
    /// JWT token for authenticated requests
    pub token: String,
}

/// Get authentication challenge endpoint
pub fn auth_challenge(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom-contract" / String / "auth-challenge")
        .and(warp::post())
        .and(context.filter())
        .and(warp::body::json())
        .and_then(handle_auth_challenge)
        .boxed()
}

/// Handle authentication challenge request
async fn handle_auth_challenge(
    contract_name: String,
    context: Context,
    body: ChallengeRequest,
) -> Result<impl Reply, Rejection> {
    debug!(
        "received custom contract '{}' auth challenge request: {:?}",
        contract_name, body
    );

    // Verify the contract exists before issuing a challenge
    if context.get_custom_contract(&contract_name).is_none() {
        warn!("custom contract '{}' not configured", contract_name);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::AuthChallenge,
            CustomContractErrorType::ContractNotConfigured,
        );
        return Err(reject::custom(ServiceError::forbidden(
            ServiceErrorCode::CustomContractAuthError(
                format!("custom contract '{}' not configured", contract_name),
                body.chain_id,
            ),
        )));
    }

    // Generate random challenge nonce
    let challenge = Uuid::new_v4().to_string();

    // Store the challenge in the cache and get the expiration timestamp
    let expires_at = context.challenge_cache().store_challenge(
        &contract_name,
        &body.chain_id,
        &body.address,
        challenge.clone(),
    );

    let response = ChallengeResponse {
        challenge,
        expires_at,
    };

    Ok(reply::json(&response))
}

/// Custom contract authentication endpoint
pub fn auth(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("custom-contract" / String / "auth")
        .and(warp::post())
        .and(context.filter())
        .and(warp::body::json())
        .and_then(handle_auth)
        .boxed()
}

/// Handle custom contract authentication request
async fn handle_auth(
    contract_name: String,
    context: Context,
    body: CustomAuthRequest,
) -> Result<impl Reply, Rejection> {
    debug!(
        "received custom contract '{}' auth request for address: {}",
        contract_name, body.address
    );

    // Get the custom contract instance
    let instance = context.get_custom_contract(&contract_name).ok_or_else(|| {
        warn!("custom contract '{}' not configured", contract_name);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::Auth,
            CustomContractErrorType::ContractNotConfigured,
        );
        reject::custom(ServiceError::forbidden(
            ServiceErrorCode::CustomContractAuthError(
                format!("custom contract '{}' not configured", contract_name),
                body.chain_id,
            ),
        ))
    })?;

    // Verify and consume the challenge BEFORE signature verification
    // This prevents replay attacks and ensures the challenge was issued by this server
    context
        .challenge_cache()
        .verify_and_consume(
            &contract_name,
            &body.chain_id,
            &body.address,
            &body.challenge,
        )
        .map_err(|e| {
            warn!(
                "challenge verification failed for address {}: {}",
                body.address, e
            );
            record_custom_contract_error(
                &contract_name,
                CustomContractEndpoint::Auth,
                CustomContractErrorType::ChallengeFailed,
            );
            reject::custom(ServiceError::bad_request(
                ServiceErrorCode::CustomContractAuthError(
                    format!("invalid or expired challenge: {}", e),
                    body.chain_id,
                ),
            ))
        })?;

    // Verify the signature over the challenge
    // This is ALWAYS required - even in open telemetry mode (no on_chain_auth),
    // nodes must prove they control the claimed address
    verify_signature(&body).map_err(|e| {
        warn!("signature verification failed: {}", e);
        record_custom_contract_error(
            &contract_name,
            CustomContractEndpoint::Auth,
            CustomContractErrorType::SignatureInvalid,
        );
        reject::custom(ServiceError::bad_request(
            ServiceErrorCode::CustomContractAuthError(e.to_string(), body.chain_id),
        ))
    })?;

    // Determine node type based on trust status
    // Priority: 1) static_allowlist (config-based trust)
    //           2) on_chain_auth allowlist (on-chain trust)
    //           3) allow_unknown_nodes (untrusted)
    //           4) reject
    let node_type = if instance.is_in_static_allowlist(&body.chain_id, &body.address) {
        // Static allowlist: trusted via config without on-chain verification
        debug!(
            "address {} in static_allowlist for contract '{}' chain {}: trusted",
            body.address, contract_name, body.chain_id
        );
        NodeType::Custom(contract_name.clone())
    } else if instance.config.is_none() {
        // Open telemetry mode: no on_chain_auth configured, not in static_allowlist
        // Treat as unknown (allow_unknown_nodes is required to be true in this mode)
        debug!(
            "open telemetry mode for contract '{}': treating address {} as unknown",
            contract_name, body.address
        );
        NodeType::CustomUnknown(contract_name.clone())
    } else {
        // Standard mode: check on-chain allowlist status
        let allowlist_status =
            check_allowlist_status(&context, &contract_name, &body.address, &body.chain_id);

        match allowlist_status {
            AllowlistStatus::InAllowlist => NodeType::Custom(contract_name.clone()),

            AllowlistStatus::NotInAllowlist if instance.allow_unknown_nodes => {
                // Issue token for unknown nodes - routed to untrusted sinks
                NodeType::CustomUnknown(contract_name.clone())
            },

            AllowlistStatus::NotInAllowlist => {
                // Unknown nodes not allowed - reject
                warn!(
                    "address {} NOT in allowlist for contract '{}' and unknown nodes not allowed",
                    body.address, contract_name
                );
                record_custom_contract_error(
                    &contract_name,
                    CustomContractEndpoint::Auth,
                    CustomContractErrorType::NotInAllowlist,
                );
                return Err(reject::custom(ServiceError::forbidden(
                    ServiceErrorCode::CustomContractAuthError(
                        format!(
                            "address {} not in allowlist for contract '{}'",
                            body.address, contract_name
                        ),
                        body.chain_id,
                    ),
                )));
            },

            AllowlistStatus::CacheMiss => {
                // Can't verify - allowlist data not available yet
                record_custom_contract_error(
                    &contract_name,
                    CustomContractEndpoint::Auth,
                    CustomContractErrorType::NotInAllowlist,
                );
                return Err(reject::custom(ServiceError::forbidden(
                    ServiceErrorCode::CustomContractAuthError(
                        format!(
                            "allowlist not available for contract '{}' (cache miss)",
                            contract_name
                        ),
                        body.chain_id,
                    ),
                )));
            },
        }
    };

    // Create JWT token for the custom contract client
    // IMPORTANT: Store the contract_name in JWT (not node_type_name) for auth verification
    // This prevents cross-contract token reuse attacks where a user authenticated for
    // contract_A could use their JWT to inject data into contract_B's sinks
    let run_uuid = Uuid::new_v4();
    let token = create_jwt_token(
        context.jwt_service(),
        body.chain_id,
        body.address, // Use address as PeerId
        node_type,    // Custom for allowlisted, CustomUnknown for unknown
        0,            // epoch not relevant for custom contract clients
        run_uuid,
    )
    .map_err(|e| {
        error!("unable to create jwt token: {}", e);
        reject::custom(ServiceError::internal(
            ServiceErrorCode::CustomContractAuthError(e.to_string(), body.chain_id),
        ))
    })?;

    Ok(reply::json(&CustomAuthResponse { token }))
}

/// Verify the Ed25519 signature over the challenge
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn verify_signature(request: &CustomAuthRequest) -> Result<()> {
    // Parse the public key
    if request.public_key.len() != 32 {
        return Err(anyhow!(
            "invalid public key length: expected 32 bytes, got {}",
            request.public_key.len()
        ));
    }

    let public_key = Ed25519PublicKey::try_from(&request.public_key[..])
        .map_err(|e: CryptoMaterialError| anyhow!("invalid public key: {}", e))?;

    // Verify the public key matches the address
    let derived_address = aptos_types::account_address::from_public_key(&public_key);
    if derived_address != request.address {
        return Err(anyhow!("public key does not match claimed address"));
    }

    // Parse the signature
    let signature = Ed25519Signature::try_from(&request.signature[..])
        .map_err(|e: CryptoMaterialError| anyhow!("invalid signature: {}", e))?;

    // Verify the signature over the challenge
    signature
        .verify_arbitrary_msg(request.challenge.as_bytes(), &public_key)
        .map_err(|e| anyhow!("signature verification failed: {}", e))?;

    Ok(())
}

/// Result of checking an address against the on-chain allowlist cache.
#[derive(Debug, Clone, PartialEq, Eq)]
enum AllowlistStatus {
    /// Address is confirmed to be in the allowlist
    InAllowlist,
    /// Address is confirmed to NOT be in the allowlist
    NotInAllowlist,
    /// Cache miss - allowlist data not available (service just started, etc.)
    CacheMiss,
}

/// Verify that the address is registered in an on-chain allowlist.
///
/// Uses a simple cache lookup - the cache is kept fresh by AllowlistCacheUpdater
/// running in the background (similar to PeerSetCacheUpdater for validators).
fn check_allowlist_status(
    context: &Context,
    contract_name: &str,
    address: &AccountAddress,
    chain_id: &ChainId,
) -> AllowlistStatus {
    let cache = context.allowlist_cache();

    match cache.check_address(contract_name, chain_id, address) {
        Some(true) => {
            debug!(
                "address {} is in allowlist for contract '{}' chain {}",
                address, contract_name, chain_id
            );
            AllowlistStatus::InAllowlist
        },
        Some(false) => {
            debug!(
                "address {} is NOT in allowlist for contract '{}' chain {}",
                address, contract_name, chain_id
            );
            AllowlistStatus::NotInAllowlist
        },
        None => {
            // Cache miss - this shouldn't happen in normal operation since
            // AllowlistCacheUpdater populates the cache before requests arrive.
            // This could occur if:
            // 1. Service just started and updater hasn't run yet
            AllowlistStatus::CacheMiss
        },
    }
}

/// Fetch addresses using view function (recommended)
/// Crate-public for cache pre-warming at startup.
pub(crate) async fn fetch_addresses_via_view_function(
    rest_url: &reqwest::Url,
    function_path: &str,
    function_args: &[String],
    address_field: &str,
) -> Result<Vec<AccountAddress>> {
    use aptos_rest_client::aptos_api_types::{EntryFunctionId, ViewRequest};
    use std::str::FromStr;

    // Parse the function path into an EntryFunctionId
    let function_id = EntryFunctionId::from_str(function_path)
        .map_err(|e| anyhow!("invalid function path '{}': {}", function_path, e))?;

    // Convert string args to JSON values for the view request
    let arguments: Vec<serde_json::Value> = function_args
        .iter()
        .map(|arg| serde_json::Value::String(arg.clone()))
        .collect();

    // Create the view request
    let view_request = ViewRequest {
        function: function_id,
        type_arguments: vec![],
        arguments,
    };

    // Use the REST client to call the view function
    let client = RestClient::new(rest_url.clone());
    let response = client.view(&view_request, None).await.map_err(|e| {
        // Provide user-friendly error messages for common failures
        let error_msg = format!("{}", e);
        if error_msg.contains("404") || error_msg.contains("not found") {
            anyhow!(
                "view function '{}' does not exist or account not found on chain",
                function_path
            )
        } else if error_msg.contains("403") || error_msg.contains("Forbidden") {
            anyhow!(
                "access denied when calling view function '{}' - check API credentials",
                function_path
            )
        } else if error_msg.contains("500") || error_msg.contains("Internal Server Error") {
            anyhow!(
                "blockchain node error when calling view function '{}': {}",
                function_path,
                error_msg
            )
        } else {
            anyhow!(
                "failed to call view function '{}': {}",
                function_path,
                error_msg
            )
        }
    })?;

    // The response is a Vec<serde_json::Value>, we need to extract addresses
    let result = response.inner();

    // Check if result is empty
    if result.is_empty() {
        return Err(anyhow!(
            "view function '{}' returned empty result - function may not exist or returned no data",
            function_path
        ));
    }

    // For view functions that return a list of objects/addresses,
    // the result is typically wrapped in an array
    let result_json = serde_json::to_value(result)
        .map_err(|e| anyhow!("failed to convert view result to JSON: {}", e))?;

    // Extract addresses from result
    extract_address_list(&result_json, address_field)
}

/// Fetch addresses using get_account_resource (legacy)
/// Crate-public for cache pre-warming at startup.
pub(crate) async fn fetch_addresses_via_resource(
    rest_url: &reqwest::Url,
    resource_path: &str,
    address_field: &str,
) -> Result<Vec<AccountAddress>> {
    // Parse resource path: "0x123::module::Resource"
    let parts: Vec<&str> = resource_path.split("::").collect();
    if parts.len() != 3 {
        return Err(anyhow!(
            "invalid resource path format, expected '0xADDRESS::module::Resource', got '{}'",
            resource_path
        ));
    }

    let resource_address = AccountAddress::from_hex_literal(parts[0])
        .map_err(|e| anyhow!("invalid address in resource path '{}': {}", parts[0], e))?;
    let resource_type = format!(
        "{}::{}::{}",
        resource_address.to_hex_literal(),
        parts[1],
        parts[2]
    );

    let client = RestClient::new(rest_url.clone());

    // Fetch the resource from on-chain
    let response = client
        .get_account_resource(resource_address, &resource_type)
        .await
        .map_err(|e| {
            // Provide user-friendly error messages for common failures
            let error_msg = format!("{}", e);
            if error_msg.contains("404") || error_msg.contains("not found") {
                anyhow!(
                    "resource '{}' does not exist at address {} or account not found on chain",
                    resource_type,
                    resource_address
                )
            } else if error_msg.contains("403") || error_msg.contains("Forbidden") {
                anyhow!(
                    "access denied when fetching resource '{}' - check API credentials",
                    resource_type
                )
            } else if error_msg.contains("500") || error_msg.contains("Internal Server Error") {
                anyhow!(
                    "blockchain node error when fetching resource '{}': {}",
                    resource_type,
                    error_msg
                )
            } else {
                anyhow!(
                    "failed to fetch resource '{}': {}",
                    resource_type,
                    error_msg
                )
            }
        })?;

    let resource = response
        .into_inner()
        .ok_or_else(|| {
            anyhow!(
                "resource '{}' not found at address {} - the resource type may not exist or account may not have this resource",
                resource_type, resource_address
            )
        })?;

    // Extract the address list from the specified field path
    extract_address_list(&resource.data, address_field)
        .map_err(|e| {
            anyhow!(
                "failed to extract addresses from resource '{}' using field path '{}': {} - check that the field exists and contains valid address data",
                resource_type, address_field, e
            )
        })
}

/// Extract a list of addresses from a JSON value using a field path
/// Supports:
/// - "providers" - simple field
/// - "data.providers" - nested path
/// - "[0].address" - array at root, extract address field from each object
/// - "" or "." - root is the array
fn extract_address_list(json: &serde_json::Value, field_path: &str) -> Result<Vec<AccountAddress>> {
    // Handle special case for array iteration syntax: "[0].field"
    if field_path.starts_with("[0].") || field_path.starts_with("[].") {
        let field_in_object = field_path.split('.').nth(1).unwrap_or("address");
        return extract_addresses_from_array_of_objects(json, field_in_object);
    }

    // Handle empty or root path
    if field_path.is_empty() || field_path == "." {
        return extract_addresses_from_value(json);
    }

    // Navigate the field path
    let mut current = json;
    for field in field_path.split('.') {
        if field.is_empty() {
            continue;
        }
        current = current
            .get(field)
            .ok_or_else(|| anyhow!("field '{}' not found in path '{}'", field, field_path))?;
    }

    extract_addresses_from_value(current)
}

/// Extract addresses from a JSON value (array or single)
#[cfg_attr(test, allow(dead_code))]
pub(crate) fn extract_addresses_from_value(
    json: &serde_json::Value,
) -> Result<Vec<AccountAddress>> {
    // The value should be an array of addresses
    let array = json
        .as_array()
        .ok_or_else(|| anyhow!("expected an array, got {:?}", json))?;

    // Parse each element as an address (either string or object with address field)
    let mut addresses = Vec::new();
    for (i, item) in array.iter().enumerate() {
        let addr = if let Some(addr_str) = item.as_str() {
            // Direct string address
            AccountAddress::from_hex_literal(addr_str)
                .map_err(|e| anyhow!("invalid address at index {}: {}", i, e))?
        } else if let Some(obj) = item.as_object() {
            // Object with 'address' or 'account' field
            let addr_str = obj
                .get("address")
                .or_else(|| obj.get("account"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("address not found in object at index {}", i))?;
            AccountAddress::from_hex_literal(addr_str)
                .map_err(|e| anyhow!("invalid address at index {}: {}", i, e))?
        } else {
            return Err(anyhow!("invalid address format at index {}", i));
        };
        addresses.push(addr);
    }

    Ok(addresses)
}

/// Extract addresses from an array of objects, pulling specific field from each
fn extract_addresses_from_array_of_objects(
    json: &serde_json::Value,
    address_field: &str,
) -> Result<Vec<AccountAddress>> {
    let outer_array = json
        .as_array()
        .ok_or_else(|| anyhow!("expected an array at root"))?;

    // View function results are wrapped: [[{...}]] - first element is the actual return value
    // If the first element is also an array, unwrap it
    let array = if outer_array.len() == 1 {
        if let Some(inner) = outer_array.first().and_then(|v| v.as_array()) {
            inner
        } else {
            outer_array
        }
    } else {
        outer_array
    };

    let mut addresses = Vec::new();
    for (i, item) in array.iter().enumerate() {
        let obj = item
            .as_object()
            .ok_or_else(|| anyhow!("expected object at index {}, got {:?}", i, item))?;
        let addr_str = obj
            .get(address_field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow!(
                    "field '{}' not found in object at index {}",
                    address_field,
                    i
                )
            })?;
        let addr = AccountAddress::from_hex_literal(addr_str)
            .map_err(|e| anyhow!("invalid address at index {}: {}", i, e))?;
        addresses.push(addr);
    }

    Ok(addresses)
}

/// Get the REST API URL for a given chain
#[allow(dead_code)]
fn get_rest_url_for_chain(chain_id: &ChainId) -> Result<reqwest::Url> {
    // Check environment variable first
    let env_var = format!("APTOS_REST_URL_CHAIN_{}", chain_id.id());
    if let Ok(url) = std::env::var(&env_var) {
        return reqwest::Url::parse(&url).map_err(|e| anyhow!("invalid URL in {}: {}", env_var, e));
    }

    // Default URLs for known chains
    let url_str = match chain_id.id() {
        1 => "https://api.mainnet.aptoslabs.com/v1",
        2 => "https://api.testnet.aptoslabs.com/v1",
        _ => {
            return Err(anyhow!(
                "unknown chain_id {}, set {} environment variable",
                chain_id.id(),
                env_var
            ))
        },
    };

    reqwest::Url::parse(url_str).map_err(|e| anyhow!("invalid default URL: {}", e))
}

/// Authorization filter for custom contract authenticated requests.
/// Returns (contract_name, peer_id, chain_id, is_trusted) from JWT claims.
/// The contract_name is extracted from NodeType::Custom or NodeType::CustomUnknown.
/// is_trusted is true for Custom (allowlisted), false for CustomUnknown.
pub fn with_custom_contract_auth(
    context: Context,
) -> impl Filter<Extract = ((String, aptos_types::PeerId, ChainId, bool),), Error = Rejection> + Clone
{
    warp::header::optional(AUTHORIZATION.as_str())
        .and_then(crate::jwt_auth::jwt_from_header)
        .and(warp::any().map(move || context.clone()))
        .and_then(authorize_custom_contract_jwt)
}

/// Authorize a custom contract JWT token.
/// Returns (contract_name, peer_id, chain_id, is_trusted) from claims for use in data ingestion.
/// The contract_name is stored in NodeType::Custom or NodeType::CustomUnknown to prevent
/// cross-contract token reuse attacks.
/// is_trusted indicates whether the node is in the on-chain allowlist (Custom) or not (CustomUnknown).
async fn authorize_custom_contract_jwt(
    token: String,
    context: Context,
) -> Result<(String, aptos_types::PeerId, ChainId, bool), Rejection> {
    use crate::types::auth::Claims;

    let claims = context
        .jwt_service()
        .decode::<Claims>(&token)
        .map_err(|e| {
            debug!("jwt decode error: {}", e);
            reject::custom(ServiceError::unauthorized(
                ServiceErrorCode::CustomContractAuthError(
                    "invalid token".to_string(),
                    ChainId::test(),
                ),
            ))
        })?
        .claims;

    // Extract and verify the contract_name from NodeType::Custom or NodeType::CustomUnknown
    // This prevents cross-contract token reuse attacks
    // CustomUnknown nodes are allowed but will be routed to untrusted sinks
    let (jwt_contract_name, is_trusted) = match &claims.node_type {
        NodeType::Custom(name) => (name.clone(), true),
        NodeType::CustomUnknown(name) => (name.clone(), false),
        _ => {
            return Err(reject::custom(ServiceError::forbidden(
                ServiceErrorCode::CustomContractAuthError(
                    "token is not for a custom contract client".to_string(),
                    claims.chain_id,
                ),
            )));
        },
    };

    Ok((
        jwt_contract_name,
        claims.peer_id,
        claims.chain_id,
        is_trusted,
    ))
}
