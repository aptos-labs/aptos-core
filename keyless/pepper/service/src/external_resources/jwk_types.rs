// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{error::PepperServiceError, external_resources::jwk_fetcher};
use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_types::jwks::rsa::RSA_JWK;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr, sync::Arc};

// Useful type declarations
pub type Issuer = String;
pub type KeyID = String;
pub type JWKCache = Arc<Mutex<HashMap<Issuer, HashMap<KeyID, Arc<RSA_JWK>>>>>;

/// A struct representing federated JWK issuers
#[derive(Clone)]
pub struct FederatedJWKs<T: FederatedJWKIssuerInterface> {
    issuers: Arc<Mutex<Vec<T>>>,
}

impl<T: FederatedJWKIssuerInterface + Clone> FederatedJWKs<T> {
    pub fn new(issuers: Vec<T>) -> Self {
        FederatedJWKs {
            issuers: Arc::new(Mutex::new(issuers)),
        }
    }

    #[cfg(test)]
    /// Creates an empty struct (for testing purposes)
    pub fn new_empty() -> Self {
        FederatedJWKs {
            issuers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns the list of federated JWK issuers
    pub fn get_issuers(&self) -> Vec<T> {
        self.issuers.lock().clone()
    }
}

/// A common interface offered by Federated JWK issuers (this is useful for logging and testing)
#[async_trait::async_trait]
pub trait FederatedJWKIssuerInterface {
    /// Returns the name of the issuer
    fn issuer_name(&self) -> String;

    /// Fetches the JWKs from the issuer's JWK URL
    async fn fetch_jwks(&self, jwt_issuer: String) -> Result<HashMap<KeyID, Arc<RSA_JWK>>>;

    /// Returns the regex used to identify the issuer
    fn regex(&self) -> &Regex;
}

/// A simple Federated JWK issuer struct
#[derive(Clone, Debug)]
pub struct FederatedJWKIssuer {
    issuer_name: String,
    issuer_jwk_url_suffix: String,
    regex: Regex,
}

impl FederatedJWKIssuer {
    pub fn new(issuer_name: String, issuer_jwk_url_suffix: String, regex: String) -> Self {
        // Create the regex
        let regex = Regex::new(&regex).unwrap_or_else(|error| {
            panic!(
                "Failed to compile federated JWK issuer regex for {}! Error: {}",
                issuer_name, error
            )
        });

        // Create the struct
        FederatedJWKIssuer {
            issuer_name,
            issuer_jwk_url_suffix,
            regex,
        }
    }
}

#[async_trait::async_trait]
impl FederatedJWKIssuerInterface for FederatedJWKIssuer {
    fn issuer_name(&self) -> String {
        self.issuer_name.clone()
    }

    async fn fetch_jwks(&self, jwt_issuer: String) -> Result<HashMap<KeyID, Arc<RSA_JWK>>> {
        let jwk_url = format!("{}{}", jwt_issuer, self.issuer_jwk_url_suffix);
        jwk_fetcher::fetch_jwks(&jwk_url).await
    }

    fn regex(&self) -> &Regex {
        &self.regex
    }
}

/// A common interface offered by JWK issuers (this is useful for logging and testing)
#[async_trait::async_trait]
pub trait JWKIssuerInterface {
    /// Returns the name of the issuer
    fn issuer_name(&self) -> String;

    /// Returns the JWK URL of the issuer
    fn issuer_jwk_url(&self) -> String;

    /// Fetches the JWKs from the issuer's JWK URL
    async fn fetch_jwks(&self) -> Result<HashMap<KeyID, Arc<RSA_JWK>>>;
}

/// A simple JWK issuer struct
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JWKIssuer {
    issuer_name: String,
    issuer_jwk_url: String,
}

impl JWKIssuer {
    pub fn new(issuer_name: String, issuer_jwk_url: String) -> JWKIssuer {
        JWKIssuer {
            issuer_name,
            issuer_jwk_url,
        }
    }

    /// Returns the name of the issuer
    pub fn issuer_name(&self) -> String {
        self.issuer_name.clone()
    }
}

#[async_trait::async_trait]
impl JWKIssuerInterface for JWKIssuer {
    fn issuer_name(&self) -> String {
        self.issuer_name.clone()
    }

    fn issuer_jwk_url(&self) -> String {
        self.issuer_jwk_url.clone()
    }

    async fn fetch_jwks(&self) -> Result<HashMap<KeyID, Arc<RSA_JWK>>> {
        jwk_fetcher::fetch_jwks(&self.issuer_jwk_url).await
    }
}

impl FromStr for JWKIssuer {
    type Err = PepperServiceError;

    /// This is used to parse each jwk issuer from the command line.
    /// The expected format is: "<iss> <jwk_url>".
    /// NOTE: we assume there is no whitespace character in either `iss` or `jwk_url`.
    fn from_str(string: &str) -> std::result::Result<Self, Self::Err> {
        // Split the string by whitespace
        let mut iterator = string.split_whitespace();

        // Parse the substrings as issuer and aud
        let issuer_name = iterator.next().ok_or(PepperServiceError::UnexpectedError(
            "Failed to parse JWK issuer name!".into(),
        ))?;
        let issuer_jwk_url = iterator.next().ok_or(PepperServiceError::UnexpectedError(
            "Failed to parse JWK issuer URL!".into(),
        ))?;

        // Verify that there are exactly 2 substrings
        if iterator.next().is_some() {
            return Err(PepperServiceError::UnexpectedError(
                "Too many arguments found for JWK issuer!".into(),
            ));
        }

        // Create the override
        let jwk_issuer = JWKIssuer::new(issuer_name.to_string(), issuer_jwk_url.to_string());
        Ok(jwk_issuer)
    }
}
