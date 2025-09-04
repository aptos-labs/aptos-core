// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod humio;

pub mod victoria_metrics_api {

    use anyhow::{anyhow, Result};
    use reqwest::{header::CONTENT_ENCODING, Client as ReqwestClient};
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
    use url::Url;
    use warp::hyper::body::Bytes;

    #[derive(Clone)]
    pub enum AuthToken {
        Bearer(String),
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

    /// Client implementation to export metrics to Victoria Metrics
    #[derive(Clone)]
    pub struct Client {
        inner: ClientWithMiddleware,
        base_url: Url,
        auth_token: AuthToken,
    }

    impl Client {
        pub fn new(base_url: Url, auth_token: AuthToken) -> Self {
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

        pub fn is_selfhosted_vm_client(&self) -> bool {
            self.base_url
                .host_str()
                .unwrap_or_default()
                .contains("velor-all.vm")
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

            let req = self
                .inner
                .post(format!("{}api/v1/import/prometheus", self.base_url));
            let req = match &self.auth_token {
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
    }
}

pub mod big_query {
    use gcp_bigquery_client::{
        error::BQError,
        model::{
            table_data_insert_all_request::TableDataInsertAllRequest,
            table_data_insert_all_response::TableDataInsertAllResponse,
        },
        Client as BigQueryClient,
    };

    #[derive(Clone)]
    pub struct TableWriteClient {
        client: BigQueryClient,
        project_id: String,
        dataset_id: String,
        table_id: String,
    }

    impl TableWriteClient {
        pub fn new(
            client: BigQueryClient,
            project_id: String,
            dataset_id: String,
            table_id: String,
        ) -> Self {
            Self {
                client,
                project_id,
                dataset_id,
                table_id,
            }
        }

        pub async fn insert_all(
            &self,
            insert_request: TableDataInsertAllRequest,
        ) -> Result<TableDataInsertAllResponse, BQError> {
            self.client
                .tabledata()
                .insert_all(
                    &self.project_id,
                    &self.dataset_id,
                    &self.table_id,
                    insert_request,
                )
                .await
        }
    }
}
