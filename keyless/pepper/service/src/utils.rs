// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::error::PepperServiceError;
use anyhow::{anyhow, ensure};
use hyper::{Body, Request};
use reqwest::{header::HeaderMap, Client};
use std::{str::FromStr, time::Duration};

// Timeout for client requests
const CLIENT_REQUEST_TIMEOUT_SECS: u64 = 15;

// Origin header constants
const MISSING_ORIGIN_STRING: &str = ""; // Default to empty string if origin header is missing
const ORIGIN_HEADER: &str = "origin";

/// An HTTP header key-value pair, parsed from CLI args in the format "name value".
#[derive(Clone, Debug)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

impl FromStr for HttpHeader {
    type Err = PepperServiceError;

    /// Parses a header from the format "header-name header-value".
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let mut iterator = string.splitn(2, ' ');

        let name = iterator
            .next()
            .ok_or(PepperServiceError::UnexpectedError(
                "Failed to parse HTTP header name".into(),
            ))?
            .to_string();
        let value = iterator
            .next()
            .ok_or(PepperServiceError::UnexpectedError(
                "Failed to parse HTTP header value".into(),
            ))?
            .to_string();

        Ok(HttpHeader { name, value })
    }
}

/// Converts a list of `HttpHeader` into a `reqwest::header::HeaderMap`.
pub fn http_headers_to_header_map(headers: &[HttpHeader]) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for header in headers {
        let name = reqwest::header::HeaderName::from_bytes(header.name.as_bytes())
            .unwrap_or_else(|e| panic!("Invalid header name '{}': {}", header.name, e));
        let value = reqwest::header::HeaderValue::from_str(&header.value)
            .unwrap_or_else(|e| panic!("Invalid value for header '{}': {}", header.name, e));
        header_map.insert(name, value);
    }
    header_map
}

/// Creates and returns a reqwest HTTP client with a timeout and the given default headers.
pub fn create_request_client(default_headers: HeaderMap) -> Client {
    Client::builder()
        .timeout(Duration::from_secs(CLIENT_REQUEST_TIMEOUT_SECS))
        .default_headers(default_headers)
        .build()
        .expect("Failed to build the request client!")
}

/// Extracts the origin header from the request
pub fn get_request_origin(request: &Request<Body>) -> String {
    request
        .headers()
        .get(ORIGIN_HEADER)
        .and_then(|header_value| header_value.to_str().ok())
        .unwrap_or(MISSING_ORIGIN_STRING)
        .to_owned()
}

/// Converts a hex-encoded string (with "0x" prefix) to a byte vector
pub fn unhexlify_api_bytes(api_output: &str) -> anyhow::Result<Vec<u8>> {
    // Verify the input format
    ensure!(api_output.len() >= 2);
    let lower = api_output.to_lowercase();
    ensure!(&lower[0..2] == "0x");

    // Decode the hex string
    hex::decode(&lower[2..]).map_err(|error| {
        anyhow!(
            "unhexlify_api_bytes() failed to decode intput {}! Error: {}",
            lower,
            error
        )
    })
}

#[cfg(test)]
mod tests {
    use crate::utils::{unhexlify_api_bytes, HttpHeader};
    use std::str::FromStr;

    #[test]
    fn test_unhexlify_api_bytes() {
        // Test valid input
        assert!(unhexlify_api_bytes("0x").unwrap().is_empty());
        assert_eq!(
            vec![0x00_u8, 0x01, 0xFF],
            unhexlify_api_bytes("0x0001ff").unwrap()
        );
        assert_eq!(
            vec![0xDE_u8, 0xAD, 0xBE, 0xEF],
            unhexlify_api_bytes("0xdeadbeef").unwrap()
        );

        // Test invalid inputs
        assert!(unhexlify_api_bytes("0001ff").is_err());
        assert!(unhexlify_api_bytes("0x0001fg").is_err());
        assert!(unhexlify_api_bytes("000").is_err());
        assert!(unhexlify_api_bytes("0").is_err());
        assert!(unhexlify_api_bytes("").is_err());
    }

    #[test]
    fn test_http_header_from_str() {
        // Test valid header
        let header = HttpHeader::from_str("x-internal-application-id my-app-123").unwrap();
        assert_eq!(header.name, "x-internal-application-id");
        assert_eq!(header.value, "my-app-123");

        // Test header with spaces in value
        let header = HttpHeader::from_str("authorization Bearer some-token-value").unwrap();
        assert_eq!(header.name, "authorization");
        assert_eq!(header.value, "Bearer some-token-value");

        // Test missing value
        assert!(HttpHeader::from_str("just-a-name").is_err());
    }
}
