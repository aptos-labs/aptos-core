// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use aptos_sdk::types::account_address::AccountAddress;
use clap::Parser;
use gcp_bigquery_client::{
    error::BQError,
    model::{
        dataset::Dataset, table::Table, table_data_insert_all_request::TableDataInsertAllRequest,
        table_field_schema::TableFieldSchema, table_schema::TableSchema,
    },
    Client as BigQueryClient,
};
use log::info;
use serde::Serialize;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

use crate::check::SingleCheck;

#[derive(Debug, Parser)]
pub struct BigQueryArgs {
    /// Path to the BigQuery key file.
    #[clap(long, parse(from_os_str))]
    pub big_query_key_path: PathBuf,

    /// GCP project ID.
    #[clap(long, default_value = "analytics-test-345723")]
    pub gcp_project_id: String,

    /// BigQuery dataset ID.
    #[clap(long, default_value = "ait3_vfn_pfn_nhc")]
    pub big_query_dataset_id: String,

    /// BigQuery table ID.
    #[clap(long, default_value = "nhc_response_data")]
    pub big_query_table_id: String,

    /// If set, do not output to BigQuery.
    #[clap(long)]
    pub big_query_dry_run: bool,
}

// This struct formats the data into a format that BigQuery expects.
#[derive(Debug, Serialize)]
pub struct MyBigQueryRow {
    pub account_address: String,
    pub nhc_response_json: String,
    pub time_response_received: u64,
    pub node_type: String,
}

impl TryFrom<(AccountAddress, SingleCheck, &str)> for MyBigQueryRow {
    type Error = anyhow::Error;

    fn try_from(
        (account_address, single_check, node_type): (AccountAddress, SingleCheck, &str),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            account_address: account_address.to_string(),
            nhc_response_json: serde_json::to_string(&single_check.result)
                .context("Failed to encode result data as JSON")?,
            time_response_received: single_check.timestamp.as_secs(),
            node_type: node_type.to_string(),
        })
    }
}

/// Instead of reading whether the dataset / table exists and then conditionally
/// creating them, we just try every time and ignore the ALREADY_EXISTS error.
/// This function does that check.
fn ignore_already_exists_error<T>(result: Result<T, BQError>) -> Result<(), BQError> {
    match result {
        Ok(_) => Ok(()),
        Err(err) => match err {
            BQError::ResponseError { ref error } => {
                if error.error.status == "ALREADY_EXISTS" {
                    Ok(())
                } else {
                    Err(err)
                }
            }
            wildcard => Err(wildcard),
        },
    }
}

/// Make a table field required, as part of the table creation request.
fn make_required(mut table_field_schema: TableFieldSchema) -> TableFieldSchema {
    table_field_schema.mode = Some("REQUIRED".to_string());
    table_field_schema
}

/// Write the response data to BigQuery. Note: On the first run this fails
/// sometimes because it doesn't seem BigQuery offers read-what-you-wrote.
/// As in, it'll make the table, but then fail to insert the data because
/// the table apparently doesn't exist, but then succeed the next time.
pub async fn write_to_big_query(
    big_query_args: &BigQueryArgs,
    nhc_responses: HashMap<AccountAddress, Vec<SingleCheck>>,
    node_type: &str,
) -> Result<()> {
    let client = BigQueryClient::from_service_account_key_file(
        big_query_args
            .big_query_key_path
            .to_str()
            .context("Big query key path was invalid")?,
    )
    .await;

    info!(
        "Creating dataset if necessary: {}",
        big_query_args.big_query_dataset_id
    );

    // Create the dataset if necessary.
    ignore_already_exists_error(
        client
            .dataset()
            .create(
                Dataset::new(
                    &big_query_args.gcp_project_id,
                    &big_query_args.big_query_dataset_id,
                )
                .location("US")
                .friendly_name("NHC AIT3 1"),
            )
            .await,
    )
    .context("Failed to create the dataset")?;

    info!("Created dataset / confirmed it was there");

    info!(
        "Creating table if necessary: {}",
        big_query_args.big_query_table_id
    );

    // Create the table if necessary.
    ignore_already_exists_error(
        client
            .table()
            .create(
                Table::new(
                    &big_query_args.gcp_project_id,
                    &big_query_args.big_query_dataset_id,
                    &big_query_args.big_query_table_id,
                    TableSchema::new(vec![
                        make_required(TableFieldSchema::string("account_address")),
                        // TODO: Consider using a record instead to give it more structure.
                        make_required(TableFieldSchema::string("nhc_response_json")),
                        make_required(TableFieldSchema::timestamp("time_response_received")),
                        make_required(TableFieldSchema::string("node_type")),
                    ]),
                )
                .friendly_name("NHC response data")
                .description("NHC check responses from fn-check-client for AIT3 FN checks"),
            )
            .await,
    )
    .context("Failed to create the table")?;

    info!("Created table / confirmed it was there");

    // Build the request to send to BigQuery.
    let mut insert_request = TableDataInsertAllRequest::new();
    for (account_address, check_results) in nhc_responses {
        for single_check_result in check_results {
            insert_request.add_row(
                None,
                MyBigQueryRow::try_from((account_address, single_check_result, node_type))?,
            )?;
        }
    }

    info!("Inserting {} rows to BigQuery", insert_request.len());

    // Submit the request.
    let response = client
        .tabledata()
        .insert_all(
            &big_query_args.gcp_project_id,
            &big_query_args.big_query_dataset_id,
            &big_query_args.big_query_table_id,
            insert_request,
        )
        .await
        .context("Failed to insert data to BigQuery")?;

    // Confirm it was successful.
    if response.insert_errors.is_some() {
        bail!("Failed to insert data to BigQuery: {:?}", response);
    }

    info!("Inserted data to BigQuery successfully");

    Ok(())
}
