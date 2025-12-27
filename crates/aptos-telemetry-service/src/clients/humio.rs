// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::types::humio::UnstructuredLog;
use anyhow::anyhow;
use debug_ignore::DebugIgnore;
use flate2::{write::GzEncoder, Compression};
use reqwest::{Client as ReqwestClient, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

pub const PEER_ID_FIELD_NAME: &str = "peer_id";
pub const EPOCH_FIELD_NAME: &str = "epoch";
pub const PEER_ROLE_TAG_NAME: &str = "peer_role";
pub const CHAIN_ID_TAG_NAME: &str = "chain_id";
pub const RUN_UUID_TAG_NAME: &str = "run_uuid";

/// Authentication configuration for Humio
#[derive(Clone, Debug)]
pub enum HumioAuth {
    /// Bearer token authentication (default for Humio)
    Bearer(String),
    /// Basic authentication (username, password)
    Basic(String, String),
}

impl HumioAuth {
    /// Create basic auth from "username:password" string
    pub fn from_basic_auth_string(creds: &str) -> Option<Self> {
        let parts: Vec<&str> = creds.splitn(2, ':').collect();
        if parts.len() == 2 {
            Some(HumioAuth::Basic(parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct IngestClient {
    inner: DebugIgnore<ClientWithMiddleware>,
    base_url: Url,
    auth: HumioAuth,
}

impl IngestClient {
    /// Create a new Humio ingest client with bearer token (backward compatible)
    pub fn new(base_url: Url, auth_token: String) -> Self {
        Self::with_auth(base_url, HumioAuth::Bearer(auth_token))
    }

    /// Create a new Humio ingest client with custom auth configuration
    pub fn with_auth(base_url: Url, auth: HumioAuth) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner: DebugIgnore(inner),
            base_url,
            auth,
        }
    }

    pub async fn ingest_unstructured_log(
        &self,
        unstructured_log: UnstructuredLog,
    ) -> Result<reqwest::Response, anyhow::Error> {
        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        serde_json::to_writer(&mut gzip_encoder, &vec![unstructured_log])
            .map_err(|e| anyhow!("unable to serialize json: {}", e))?;
        let compressed_bytes = gzip_encoder.finish()?;

        let req = self
            .inner
            .0
            .post(self.base_url.join("api/v1/ingest/humio-unstructured")?)
            .header("Content-Encoding", "gzip")
            .body(compressed_bytes);

        // Add authentication based on configured auth type
        let req = match &self.auth {
            HumioAuth::Bearer(token) => req.bearer_auth(token),
            HumioAuth::Basic(username, password) => req.basic_auth(username, Some(password)),
        };

        req.send()
            .await
            .map_err(|e| anyhow!("failed to post logs: {}", e))
    }
}
