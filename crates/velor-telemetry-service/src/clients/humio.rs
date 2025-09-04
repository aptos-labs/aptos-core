// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::humio::UnstructuredLog;
use anyhow::anyhow;
use flate2::{write::GzEncoder, Compression};
use reqwest::{Client as ReqwestClient, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

pub const PEER_ID_FIELD_NAME: &str = "peer_id";
pub const EPOCH_FIELD_NAME: &str = "epoch";
pub const PEER_ROLE_TAG_NAME: &str = "peer_role";
pub const CHAIN_ID_TAG_NAME: &str = "chain_id";
pub const RUN_UUID_TAG_NAME: &str = "run_uuid";

#[derive(Clone)]
pub struct IngestClient {
    inner: ClientWithMiddleware,
    base_url: Url,
    auth_token: String,
}

impl IngestClient {
    pub fn new(base_url: Url, auth_token: String) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner,
            base_url,
            auth_token,
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

        self.inner
            .post(self.base_url.join("api/v1/ingest/humio-unstructured")?)
            .bearer_auth(self.auth_token.clone())
            .header("Content-Encoding", "gzip")
            .body(compressed_bytes)
            .send()
            .await
            .map_err(|e| anyhow!("failed to post metrics: {}", e))
    }
}
