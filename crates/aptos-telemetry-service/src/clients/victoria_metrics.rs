// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Result};
use debug_ignore::DebugIgnore;
use reqwest::{header::CONTENT_ENCODING, Client as ReqwestClient};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use url::Url;
use warp::hyper::body::Bytes;

#[derive(Clone, Debug)]
pub enum AuthToken {
    /// No authentication - skips adding any Authorization header
    None,
    /// Bearer token authentication
    Bearer(String),
    /// Basic authentication (username, password)
    Basic(String, String),
}

impl From<&String> for AuthToken {
    fn from(token: &String) -> Self {
        // TODO(ibalajiarun): Auth type must be read from config
        if token.split(':').count() == 2 {
            let mut parts = token.split(':');
            AuthToken::Basic(
                parts.next().unwrap().to_string(),
                parts.next().unwrap().to_string(),
            )
        } else {
            AuthToken::Bearer(token.to_string())
        }
    }
}

impl From<&str> for AuthToken {
    fn from(token: &str) -> Self {
        AuthToken::from(&token.to_string())
    }
}

/// Client to push metrics to Victoria Metrics
#[derive(Clone, Debug)]
pub struct VictoriaMetricsClient {
    inner: DebugIgnore<ClientWithMiddleware>,
    base_url: Url,
    auth_token: AuthToken,
}

impl VictoriaMetricsClient {
    pub fn new(base_url: Url, auth_token: AuthToken) -> Self {
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let inner = ClientBuilder::new(ReqwestClient::new())
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();
        Self {
            inner: DebugIgnore(inner),
            base_url,
            auth_token,
        }
    }

    pub fn is_selfhosted_vm_client(&self) -> bool {
        self.base_url
            .host_str()
            .unwrap_or_default()
            .contains("aptos-all.vm")
    }

    pub async fn post_prometheus_metrics(
        &self,
        raw_metrics_body: Bytes,
        extra_labels: Vec<String>,
        encoding: String,
    ) -> Result<reqwest::Response, anyhow::Error> {
        let labels: Vec<(String, String)> = extra_labels
            .iter()
            .map(|label| ("extra_label".into(), label.into()))
            .collect();

        // Use base_url directly (config should include full endpoint path)
        let req = self.inner.0.post(self.base_url.as_str());
        let req = match &self.auth_token {
            AuthToken::None => req,
            AuthToken::Bearer(token) => req.bearer_auth(token.clone()),
            AuthToken::Basic(username, password) => {
                req.basic_auth(username.clone(), Some(password.clone()))
            },
        };

        req.header(CONTENT_ENCODING, encoding)
            .query(&labels)
            .body(raw_metrics_body)
            .send()
            .await
            .map_err(|e| anyhow!("failed to post metrics: {}", e))
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }
}
