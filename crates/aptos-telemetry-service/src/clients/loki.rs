// Copyright © Aptos Foundation

use crate::types::loki::LokiLog;
use flate2::{write::GzEncoder, Compression};
use reqwest::{Client as ReqwestClient, Url};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};

#[derive(Clone)]
pub struct LokiClient {
    inner: ClientWithMiddleware,
    base_url: Url,
    basic_user: String,
    basic_password: String,
}

impl LokiClient {
    pub fn new(base_url: Url, basic_user: String, basic_password: String) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner,
            base_url,
            basic_user,
            basic_password,
        }
    }

    pub async fn ingest_log(&self, log: LokiLog) -> Result<reqwest::Response, anyhow::Error> {
        let mut gzip_encoder = GzEncoder::new(Vec::new(), Compression::default());
        serde_json::to_writer(&mut gzip_encoder, &log)
            .map_err(|e| anyhow::anyhow!("unable to serialize json: {}", e))?;
        let compressed_bytes = gzip_encoder.finish()?;

        self.inner
            .post(self.base_url.join("/loki/api/v1/push")?)
            .basic_auth(self.basic_user.clone(), Some(self.basic_password.clone()))
            .header("Content-Type", "application/json")
            .header("Content-Encoding", "gzip")
            .body(compressed_bytes)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("failed to post metrics: {}", e))
    }
}
