// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::external_resources::{
    jwk_fetcher::{
        get_federated_jwk, AUTH0_ISSUER_NAME, AUTH0_REGEX_STR, COGNITO_ISSUER_NAME,
        COGNITO_REGEX_STR,
    },
    jwk_types::{FederatedJWKIssuerInterface, FederatedJWKs, KeyID},
};
use aptos_keyless_pepper_common::jwt::Claims;
use aptos_types::{
    jwks::rsa::{INSECURE_TEST_RSA_JWK, RSA_JWK},
    keyless,
};
use regex::Regex;
use std::{collections::HashMap, ops::Deref, sync::Arc};

/// A simple JWT payload for testing purposes
#[derive(Debug)]
pub struct TestJWTPayload {
    pub claims: Claims, // The claims used to build the JWT
    pub kid: String,    // The key ID placed in the JWT header
}

impl TestJWTPayload {
    pub fn new(kid: String, issuer: String) -> Self {
        // Create an empty claims for testing
        let claims = Claims {
            nonce: "".into(),
            iss: issuer,
            sub: "".into(),
            email: None,
            azp: None,
            aud: "".into(),
            iat: 0,
            exp: 0,
        };

        Self { claims, kid }
    }

    /// Returns the base64-encoded JWT string
    pub fn get_base64_encoded_jwt(&self) -> String {
        // Create the JWT header
        let header = serde_json::json!({
            "alg": "RS256",
            "typ": "JWT",
            "kid": self.kid,
        });

        // Base64url encode the header and claims
        let header_b64 = keyless::base64url_encode_str(&serde_json::to_string(&header).unwrap());
        let claims_b64 =
            keyless::base64url_encode_str(&serde_json::to_string(&self.claims).unwrap());

        // Construct the raw JWT
        format!("{}.{}.sig", header_b64, claims_b64)
    }
}

/// A mock federated JWK issuer (for testing JWK regex matching and fetching)
#[derive(Clone, Debug)]
struct MockFederatedJWKIssuer {
    issuer_name: String,
    jwks: HashMap<KeyID, Arc<RSA_JWK>>,
    regex: Regex,
}

impl MockFederatedJWKIssuer {
    pub fn new(
        issuer_name: String,
        jwks: HashMap<KeyID, Arc<RSA_JWK>>,
        regex_pattern: String,
    ) -> Self {
        let regex = Regex::new(&regex_pattern).expect("Failed to create regex!");
        Self {
            issuer_name,
            jwks,
            regex,
        }
    }
}

#[async_trait::async_trait]
impl FederatedJWKIssuerInterface for MockFederatedJWKIssuer {
    fn issuer_name(&self) -> String {
        self.issuer_name.clone()
    }

    async fn fetch_jwks(
        &self,
        _jwt_issuer: String,
    ) -> anyhow::Result<HashMap<KeyID, Arc<RSA_JWK>>> {
        Ok(self.jwks.clone())
    }

    fn regex(&self) -> &Regex {
        &self.regex
    }
}

#[tokio::test]
async fn test_federated_jwk_fetch() {
    // Create test jwks to be returned by the mock issuer
    let mut test_jwks: HashMap<KeyID, Arc<RSA_JWK>> = HashMap::new();
    let test_kid = "test_kid";
    let test_rsa_jwk = Arc::new(INSECURE_TEST_RSA_JWK.deref().clone());
    test_jwks.insert(test_kid.into(), test_rsa_jwk.clone());

    // Create the mock federated JWK issuer
    let mock_issuer =
        MockFederatedJWKIssuer::new(AUTH0_ISSUER_NAME.into(), test_jwks, AUTH0_REGEX_STR.into());
    let federated_jwks = FederatedJWKs::new(vec![mock_issuer]);

    // Create the test JWT payload with a valid auth0 issuer
    let iss = "https://test.us.auth0.com/";
    let jwt_payload = TestJWTPayload::new(test_kid.into(), iss.into());

    // Fetch the federated JWK
    let federated_jwk_result =
        get_federated_jwk(&jwt_payload.get_base64_encoded_jwt(), federated_jwks).await;

    // Verify that the JWK was fetched successfully
    assert_eq!(federated_jwk_result.unwrap(), test_rsa_jwk);
}

#[tokio::test]
async fn test_federated_jwk_fetch_multiple_issuers() {
    // Create test jwks to be returned by the third issuer
    let mut test_jwks: HashMap<KeyID, Arc<RSA_JWK>> = HashMap::new();
    let test_kid = "falcon_kid";
    let test_rsa_jwk = Arc::new(INSECURE_TEST_RSA_JWK.deref().clone());
    test_jwks.insert(test_kid.into(), test_rsa_jwk.clone());

    // Create several mock federated JWK issuers
    let mock_issuer_1 = MockFederatedJWKIssuer::new(
        AUTH0_ISSUER_NAME.into(),
        HashMap::new(), // Empty JWKs
        AUTH0_REGEX_STR.into(),
    );
    let mock_issuer_2 = MockFederatedJWKIssuer::new(
        COGNITO_ISSUER_NAME.into(),
        HashMap::new(), // Empty JWKs
        COGNITO_REGEX_STR.into(),
    );
    let mock_issuer_3 = MockFederatedJWKIssuer::new(
        "falcon".into(),
        test_jwks,
        r"^https://[a-zA-Z0-9_-]+\.falcon\.com/".into(),
    );
    let federated_jwks = FederatedJWKs::new(vec![mock_issuer_1, mock_issuer_2, mock_issuer_3]);

    // Create the test JWT payload with an issuer that matches the third issuer
    let iss = "https://example.falcon.com/";
    let jwt_payload = TestJWTPayload::new(test_kid.into(), iss.into());

    // Fetch the federated JWK
    let federated_jwk_result =
        get_federated_jwk(&jwt_payload.get_base64_encoded_jwt(), federated_jwks).await;

    // Verify that the JWK was fetched successfully
    assert_eq!(federated_jwk_result.unwrap(), test_rsa_jwk);
}

#[tokio::test]
async fn test_federated_jwk_fetch_fails_for_bad_issuer() {
    // Create the mock federated JWK issuer
    let mock_issuer = MockFederatedJWKIssuer::new(
        AUTH0_ISSUER_NAME.into(),
        HashMap::new(), // Empty JWKs
        AUTH0_REGEX_STR.into(),
    );
    let federated_jwks = FederatedJWKs::new(vec![mock_issuer]);

    // Create the test JWT payload with a bad issuer
    let iss = "https://test.us.random.com/";
    let kid = "kid";
    let jwt_payload = TestJWTPayload::new(kid.into(), iss.into());

    // Fetch the federated JWK
    let federated_jwk_result =
        get_federated_jwk(&jwt_payload.get_base64_encoded_jwt(), federated_jwks).await;

    // Verify that the JWK issuer was not found
    let error_message = federated_jwk_result.unwrap_err().to_string();
    assert!(error_message.contains("Unsupported federated issuer: https://test.us.random.com/"));
}

#[tokio::test]
async fn test_federated_jwk_fetch_fails_for_missing_kid() {
    // Create the mock federated JWK issuer
    let mock_issuer = MockFederatedJWKIssuer::new(
        AUTH0_ISSUER_NAME.into(),
        HashMap::new(), // Empty JWKs
        AUTH0_REGEX_STR.into(),
    );
    let federated_jwks = FederatedJWKs::new(vec![mock_issuer]);

    // Create the test JWT payload with a valid issuer
    let iss = "https://test.us.auth0.com/";
    let kid = "missing_kid"; // This kid will not be found
    let jwt_payload = TestJWTPayload::new(kid.into(), iss.into());

    // Fetch the federated JWK
    let federated_jwk_result =
        get_federated_jwk(&jwt_payload.get_base64_encoded_jwt(), federated_jwks).await;

    // Verify that the JWK was not found
    let error_message = federated_jwk_result.unwrap_err().to_string();
    assert!(error_message.contains("Unknown kid: missing_kid"));
}
