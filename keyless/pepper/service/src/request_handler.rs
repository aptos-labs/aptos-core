// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dedicated_handlers::handlers::{
        HandlerTrait, V0FetchHandler, V0SignatureHandler, V0VerifyHandler,
    },
    error::PepperServiceError,
    external_resources::{jwk_fetcher::JWKCache, resource_fetcher::CachedResources},
};
use aptos_build_info::build_information;
use aptos_keyless_pepper_common::BadPepperRequestError;
use aptos_logger::{error, info, warn};
use hyper::{
    header::{
        ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
        ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE,
    },
    http::response,
    Body, Method, Request, Response, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{convert::Infallible, fmt::Debug, ops::Deref, sync::Arc};

// Default port for the pepper service
pub const DEFAULT_PEPPER_SERVICE_PORT: u16 = 8000;

// The list of endpoints/paths offered by the Pepper Service.
// Note: if you update these paths, please also update the "ALL_PATHS" array below.
pub const ABOUT_PATH: &str = "/about";
pub const GROTH16_VK_PATH: &str = "/cached/groth16-vk";
pub const JWK_PATH: &str = "/cached/jwk";
pub const KEYLESS_CONFIG_PATH: &str = "/cached/keyless-config";
pub const FETCH_PATH: &str = "/v0/fetch";
pub const SIGNATURE_PATH: &str = "/v0/signature";
pub const VERIFY_PATH: &str = "/v0/verify";
pub const VUF_PUB_KEY_PATH: &str = "/v0/vuf-pub-key";

// An array of all known endpoints/paths
pub const ALL_PATHS: [&str; 8] = [
    ABOUT_PATH,
    GROTH16_VK_PATH,
    JWK_PATH,
    KEYLESS_CONFIG_PATH,
    FETCH_PATH,
    SIGNATURE_PATH,
    VERIFY_PATH,
    VUF_PUB_KEY_PATH,
];

// Content type constants
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_TEXT: &str = "text/plain";

// Origin header constants
const MISSING_ORIGIN_STRING: &str = ""; // Default to empty string if origin header is missing
const ORIGIN_HEADER: &str = "origin";

// Useful message constants
const METHOD_NOT_ALLOWED_MESSAGE: &str =
    "The request method is not allowed for the requested path!";
const UNEXPECTED_ERROR_MESSAGE: &str = "An unexpected error was encountered!";

/// Calls the dedicated request handler for the given request
async fn call_dedicated_request_handler<TRequest, TResponse, TRequestHandler>(
    origin: String,
    request: Request<Body>,
    vuf_private_key: &ark_bls12_381::Fr,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
    request_handler: &TRequestHandler,
) -> Result<Response<Body>, Infallible>
where
    TRequest: Debug + Serialize + DeserializeOwned,
    TResponse: Debug + Serialize,
    TRequestHandler: HandlerTrait<TRequest, TResponse> + Send + Sync,
{
    // Get the request body bytes
    let request_body = request.into_body();
    let request_bytes = match hyper::body::to_bytes(request_body).await {
        Ok(request_bytes) => request_bytes,
        Err(error) => {
            error!("Failed to get request body bytes: {}", error);
            return generate_internal_server_error_response(origin);
        },
    };

    // Extract the pepper request from the request bytes
    let pepper_request: TRequest = match serde_json::from_slice(&request_bytes) {
        Ok(pepper_request) => pepper_request,
        Err(error) => {
            let error_string = format!("Failed to deserialize request body JSON: {}", error);
            warn!("{}", error_string);
            return generate_bad_request_response(origin, error_string);
        },
    };

    // Invoke the handler and generate the response
    match request_handler
        .handle(vuf_private_key, jwk_cache, cached_resources, pepper_request)
        .await
    {
        Ok(pepper_response) => {
            info!(
                "Pepper request processed successfully! Origin: {}, handler: {}",
                origin,
                request_handler.get_handler_name()
            );
            let response_body = match serde_json::to_string_pretty(&pepper_response) {
                Ok(response_body) => response_body,
                Err(error) => {
                    error!("Failed to serialize response to pretty JSON: {}", error);
                    return generate_internal_server_error_response(origin);
                },
            };
            generate_json_response(origin, StatusCode::OK, response_body)
        },
        Err(PepperServiceError::BadRequest(error)) => {
            let error_string = format!(
                "Pepper request processing failed with bad request: {}",
                error
            );
            warn!("{}", error_string);

            let bad_pepper_request_error = BadPepperRequestError {
                message: error_string,
            };
            let error_message = match serde_json::to_string_pretty(&bad_pepper_request_error) {
                Ok(error_message) => error_message,
                Err(error) => {
                    error!("Failed to serialize response to pretty JSON: {}", error);
                    return generate_internal_server_error_response(origin);
                },
            };
            generate_bad_request_response(origin, error_message)
        },
        Err(PepperServiceError::InternalError(error)) => {
            error!(
                "Pepper request processing failed with internal error: {}",
                error
            );
            generate_internal_server_error_response(origin)
        },
        Err(PepperServiceError::UnexpectedError(error)) => {
            error!(
                "Pepper request processing failed with unexpected error: {}",
                error
            );
            generate_internal_server_error_response(origin)
        },
    }
}

/// Returns a response builder prepopulated with common headers
fn create_response_builder(origin: String, status_code: StatusCode) -> response::Builder {
    hyper::Response::builder()
        .status(status_code)
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, origin)
        .header(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true")
}

/// Generates a response for the about endpoint
fn generate_about_response(origin: String) -> Result<Response<Body>, Infallible> {
    // Get the build information
    let build_information = build_information!();

    // Serialize the build information
    let build_info_json = match serde_json::to_string_pretty(&build_information) {
        Ok(json) => json,
        Err(error) => {
            error!("Failed to serialize build information to JSON: {}", error);
            return generate_internal_server_error_response(origin);
        },
    };

    // Generate the response
    generate_json_response(origin, StatusCode::OK, build_info_json)
}

/// Generates a 400 response for bad requests
fn generate_bad_request_response(
    origin: String,
    json_error_string: String,
) -> Result<Response<Body>, Infallible> {
    generate_json_response(origin, StatusCode::BAD_REQUEST, json_error_string)
}

/// Generates a response for cached resources
fn generate_cached_resource_response<T: Serialize>(
    origin: String,
    request_method: &Method,
    path: &str,
    cached_resource: Option<T>,
) -> Result<Response<Body>, Infallible> {
    if let Some(cached_resource) = cached_resource {
        match serde_json::to_string(&cached_resource) {
            Ok(response_body) => generate_json_response(origin, StatusCode::OK, response_body),
            Err(error) => {
                // Failed to serialize the cached resource, return a server error
                error!("Failed to serialize to JSON response: {}", error);
                generate_internal_server_error_response(origin)
            },
        }
    } else {
        // The cached resource was not found, return a missing resource error
        generate_not_found_response(origin, request_method, path)
    }
}

/// Generates a 500 response for unexpected internal server errors
fn generate_internal_server_error_response(origin: String) -> Result<Response<Body>, Infallible> {
    generate_text_response(
        origin,
        StatusCode::INTERNAL_SERVER_ERROR,
        UNEXPECTED_ERROR_MESSAGE.into(),
    )
}

/// Generates a JSON response with the given status code and body string
fn generate_json_response(
    origin: String,
    status_code: StatusCode,
    body_str: String,
) -> Result<Response<Body>, Infallible> {
    let response = create_response_builder(origin, status_code)
        .header(CONTENT_TYPE, CONTENT_TYPE_JSON)
        .body(Body::from(body_str))
        .expect("Failed to build JSON response!");
    Ok(response)
}

/// Generates a response with the cached JWKs as JSON
fn generate_jwt_cache_response(
    origin: String,
    jwk_cache: JWKCache,
) -> Result<Response<Body>, Infallible> {
    let jwk_cache = jwk_cache.lock().deref().clone();
    match serde_json::to_string_pretty(&jwk_cache) {
        Ok(response_body) => generate_json_response(origin, StatusCode::OK, response_body),
        Err(error) => {
            // Failed to serialize the JWK cache, return a server error
            error!("Failed to serialize to JSON response: {}", error);
            generate_internal_server_error_response(origin.clone())
        },
    }
}

/// Generates a 405 response for invalid methods on known paths
fn generate_method_not_allowed_response(origin: String) -> Result<Response<Body>, Infallible> {
    generate_text_response(
        origin,
        StatusCode::METHOD_NOT_ALLOWED,
        METHOD_NOT_ALLOWED_MESSAGE.into(),
    )
}

/// Generates a 404 response for invalid paths
fn generate_not_found_response(
    origin: String,
    request_method: &Method,
    invalid_path: &str,
) -> Result<Response<Body>, Infallible> {
    let response_message = format!(
        "The request for '{}' with method '{}' was not found!",
        invalid_path, request_method
    );
    generate_text_response(origin, StatusCode::NOT_FOUND, response_message)
}

/// Generates a response for options requests
fn generate_options_response(origin: String) -> Result<Response<Body>, Infallible> {
    let response = create_response_builder(origin, StatusCode::OK)
        .header(ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, OPTIONS")
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .body(Body::empty())
        .expect("Failed to build options response!");
    Ok(response)
}

/// Extracts the origin header from the request
fn get_request_origin(request: &Request<Body>) -> String {
    request
        .headers()
        .get(ORIGIN_HEADER)
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or(MISSING_ORIGIN_STRING)
        .to_owned()
}

/// Generates a text response with the given status code and body string
fn generate_text_response(
    origin: String,
    status_code: StatusCode,
    body_str: String,
) -> Result<Response<Body>, Infallible> {
    let response = create_response_builder(origin, status_code)
        .header(CONTENT_TYPE, CONTENT_TYPE_TEXT)
        .body(Body::from(body_str))
        .expect("Failed to build text response!");
    Ok(response)
}

/// Handles the given request and returns a response
pub async fn handle_request(
    request: Request<Body>,
    vuf_keypair: Arc<(String, ark_bls12_381::Fr)>,
    jwk_cache: JWKCache,
    cached_resources: CachedResources,
) -> Result<Response<Body>, Infallible> {
    // Get the request origin
    let origin = get_request_origin(&request);

    // Handle any OPTIONS requests
    let request_method = request.method();
    if request_method == Method::OPTIONS {
        return generate_options_response(origin);
    }

    // Handle any GET requests
    if request_method == Method::GET {
        match request.uri().path() {
            ABOUT_PATH => return generate_about_response(origin),
            GROTH16_VK_PATH => {
                let groth16_vk = cached_resources.read_on_chain_groth16_vk();
                return generate_cached_resource_response(
                    origin,
                    request_method,
                    GROTH16_VK_PATH,
                    groth16_vk,
                );
            },
            JWK_PATH => {
                return generate_jwt_cache_response(origin, jwk_cache);
            },
            KEYLESS_CONFIG_PATH => {
                let keyless_config = cached_resources.read_on_chain_keyless_configuration();
                return generate_cached_resource_response(
                    origin,
                    request_method,
                    KEYLESS_CONFIG_PATH,
                    keyless_config,
                );
            },
            VUF_PUB_KEY_PATH => {
                let (vuf_pub_key, _) = vuf_keypair.deref();
                return generate_json_response(origin, StatusCode::OK, vuf_pub_key.clone());
            },
            _ => { /* Continue below */ },
        };
    }

    // Handle any POST requests
    let (_, vuf_priv_key) = vuf_keypair.deref();
    if request_method == Method::POST {
        match request.uri().path() {
            FETCH_PATH => {
                return call_dedicated_request_handler(
                    origin,
                    request,
                    vuf_priv_key,
                    jwk_cache,
                    cached_resources,
                    &V0FetchHandler,
                )
                .await
            },
            SIGNATURE_PATH => {
                return call_dedicated_request_handler(
                    origin,
                    request,
                    vuf_priv_key,
                    jwk_cache,
                    cached_resources,
                    &V0SignatureHandler,
                )
                .await
            },
            VERIFY_PATH => {
                return call_dedicated_request_handler(
                    origin,
                    request,
                    vuf_priv_key,
                    jwk_cache,
                    cached_resources,
                    &V0VerifyHandler,
                )
                .await
            },
            _ => { /* Continue below */ },
        };
    }

    // If the request is to a known path but with an invalid method, return a method not allowed response
    if is_known_path(request.uri().path()) {
        return generate_method_not_allowed_response(origin);
    }

    // Otherwise, no matching route was found
    generate_not_found_response(origin, request_method, request.uri().path())
}

/// Returns true if the given URI path is a known path/endpoint
fn is_known_path(uri_path: &str) -> bool {
    ALL_PATHS.contains(&uri_path)
}
