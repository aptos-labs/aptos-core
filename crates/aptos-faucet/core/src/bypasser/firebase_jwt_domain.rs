// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::BypasserTrait;
use crate::{
    checkers::CheckerData,
    firebase_jwt::{FirebaseJwtVerifier, FirebaseJwtVerifierConfig},
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirebaseJwtDomainBypasserConfig {
    /// Domains that may bypass rate limits when the request includes a verified JWT.
    pub allowed_domains: Vec<String>,
    /// The GCP project ID for Firebase Identity Platform.
    pub identity_platform_gcp_project: String,
}

pub struct FirebaseJwtDomainBypasser {
    allowed_domains: Vec<String>,
    verifier: FirebaseJwtVerifier,
}

impl FirebaseJwtDomainBypasser {
    pub async fn new(config: FirebaseJwtDomainBypasserConfig) -> Result<Self> {
        if config.allowed_domains.is_empty() {
            return Err(anyhow!(
                "FirebaseJwtDomainBypasser requires at least one allowed domain"
            ));
        }

        let verifier = FirebaseJwtVerifier::new(FirebaseJwtVerifierConfig {
            identity_platform_gcp_project: config.identity_platform_gcp_project,
        })
        .await?;

        Ok(Self {
            allowed_domains: config
                .allowed_domains
                .into_iter()
                .map(|domain| domain.to_ascii_lowercase())
                .collect(),
            verifier,
        })
    }
}

#[async_trait]
impl BypasserTrait for FirebaseJwtDomainBypasser {
    async fn request_can_bypass(&self, data: CheckerData) -> Result<bool> {
        let claims = match self
            .verifier
            .validate_jwt_claims(data.headers.clone())
            .await
        {
            Ok(claims) => claims,
            Err(_) => return Ok(false),
        };
        let email = claims.email.to_ascii_lowercase();
        let domain = match email.rsplit_once('@') {
            Some((_, domain)) => domain,
            None => return Ok(false),
        };
        Ok(self.allowed_domains.iter().any(|allowed| allowed == domain))
    }
}
