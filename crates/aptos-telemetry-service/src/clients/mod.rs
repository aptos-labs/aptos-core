// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod humio;

pub mod victoria_metrics_api {

    use anyhow::{anyhow, Result};

    use reqwest::{header::CONTENT_ENCODING, Client as ReqwestClient};
    use url::Url;
    use warp::hyper::body::Bytes;

    #[derive(Clone)]
    pub struct Client {
        inner: ReqwestClient,
        base_url: Url,
        auth_token: String,
    }

    impl Client {
        pub fn new(base_url: Url, auth_token: String) -> Self {
            Self {
                inner: ReqwestClient::new(),
                base_url,
                auth_token,
            }
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

            self.inner
                .post(format!("{}api/v1/import/prometheus", self.base_url))
                .bearer_auth(self.auth_token.clone())
                .header(CONTENT_ENCODING, encoding)
                .query(&labels)
                .body(raw_metrics_body)
                .send()
                .await
                .map_err(|e| anyhow!("failed to post metrics: {}", e))
        }
    }
}
