// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod humio;

pub mod victoria_metrics_api {

    use anyhow::{anyhow, Result};

    use reqwest::{header::CONTENT_ENCODING, Client as ReqwestClient};
    use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
    use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
    use url::Url;
    use warp::hyper::body::Bytes;

    /// Client implementation to export metrics to Victoria Metrics
    #[derive(Clone)]
    pub struct Client {
        inner: ClientWithMiddleware,
        base_url: Url,
        auth_token: String,
    }

    impl Client {
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
