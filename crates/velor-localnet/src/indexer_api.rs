// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use tracing::info;

pub const HASURA_IMAGE: &str = "hasura/graphql-engine:v2.44.0-ce";

/// This Hasura metadata originates from the velor-indexer-processors repo.
///
/// This metadata here should come from the same revision as the `processor` dep.
///
/// The metadata file is not taken verbatim, it is currently edited by hand to remove
/// any references to tables that aren't created by the Rust processor migrations.
///
/// To arrive at the final edited file I normally start with the new metadata file,
/// try to start the localnet, and check .velor/testnet/main/tracing.log to
/// see what error Hasura returned. Remove the culprit from the metadata, which is
/// generally a few tables and relations to those tables, and try again. Repeat until
/// it accepts the metadata.
///
/// This works fine today since all the key processors you'd need in a localnet
/// are in the set of processors written in Rust. If this changes, we can explore
/// alternatives, e.g. running processors in other languages using containers.
pub const HASURA_METADATA: &str = include_str!("hasura_metadata.json");

/// This submits a POST request to apply metadata to a Hasura API.
pub async fn post_metadata(url: Url, metadata_content: &str) -> Result<()> {
    // Parse the metadata content as JSON.
    let metadata_json: serde_json::Value = serde_json::from_str(metadata_content)?;

    // Make the request.
    info!("Submitting request to apply Hasura metadata");
    let response =
        make_hasura_metadata_request(url, "replace_metadata", Some(metadata_json)).await?;
    info!(
        "Received response for applying Hasura metadata: {:?}",
        response
    );

    // Confirm that the metadata was applied successfully and there is no inconsistency
    // between the schema and the underlying DB schema.
    if let Some(obj) = response.as_object() {
        if let Some(is_consistent_val) = obj.get("is_consistent") {
            if is_consistent_val.as_bool() == Some(true) {
                return Ok(());
            }
        }
    }

    Err(anyhow!(
        "Something went wrong applying the Hasura metadata, perhaps it is not consistent with the DB. Response: {:#?}",
        response
    ))
}

/// This confirms that the metadata has been applied. We use this in the health
/// checker.
pub async fn confirm_metadata_applied(url: Url) -> Result<()> {
    // Make the request.
    info!("Confirming Hasura metadata applied...");
    let response = make_hasura_metadata_request(url, "export_metadata", None).await?;
    info!(
        "Received response for confirming Hasura metadata applied: {:?}",
        response
    );

    // If the sources field is set it means the metadata was applied successfully.
    if let Some(obj) = response.as_object() {
        if let Some(sources) = obj.get("sources") {
            if let Some(sources) = sources.as_array() {
                if !sources.is_empty() {
                    return Ok(());
                }
            }
        }
    }

    Err(anyhow!(
        "The Hasura metadata has not been applied yet. Response: {:#?}",
        response
    ))
}

/// The /v1/metadata endpoint supports a few different operations based on the `type`
/// field in the request body. All requests have a similar format, with these `type`
/// and `args` fields.
pub async fn make_hasura_metadata_request(
    mut url: Url,
    typ: &str,
    args: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();

    // Update the query path.
    url.set_path("/v1/metadata");

    // Construct the payload.
    let mut payload = serde_json::Map::new();
    payload.insert(
        "type".to_string(),
        serde_json::Value::String(typ.to_string()),
    );

    // If args is provided, use that. Otherwise use an empty object. We have to set it
    // no matter what because the API expects the args key to be set.
    let args = match args {
        Some(args) => args,
        None => serde_json::Value::Object(serde_json::Map::new()),
    };
    payload.insert("args".to_string(), args);

    // Send the POST request.
    let response = client.post(url).json(&payload).send().await?;

    // Return the response as a JSON value.
    response
        .json()
        .await
        .context("Failed to parse response as JSON")
}
