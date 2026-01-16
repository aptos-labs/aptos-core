// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod humio;
pub mod loki;
pub mod prometheus_remote_write;
pub mod victoria_metrics;

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
