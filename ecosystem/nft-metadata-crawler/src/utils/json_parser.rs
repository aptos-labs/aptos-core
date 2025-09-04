// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_uri_metadata,
    utils::{
        constants::{MAX_JSON_REQUEST_RETRY_SECONDS, MAX_RETRY_TIME_SECONDS},
        counters::{
            FAILED_TO_PARSE_JSON_COUNT, PARSE_JSON_INVOCATION_COUNT, SUCCESSFULLY_PARSED_JSON_COUNT,
        },
    },
};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use image::ImageFormat;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tracing::info;

pub struct JSONParser;

impl JSONParser {
    /// Parses JSON from input URI.
    /// Returns the underlying raw image URI, raw animation URI, and JSON.
    pub async fn parse(
        uri: String,
        max_file_size_bytes: u32,
    ) -> anyhow::Result<(Option<String>, Option<String>, Value)> {
        PARSE_JSON_INVOCATION_COUNT.inc();
        let (mime, size) = get_uri_metadata(&uri).await?;
        if ImageFormat::from_mime_type(&mime).is_some() {
            FAILED_TO_PARSE_JSON_COUNT
                .with_label_values(&["found image instead"])
                .inc();
            return Err(anyhow::anyhow!(format!(
                "JSON parser received image file: {}, skipping",
                mime
            )));
        } else if size > max_file_size_bytes {
            FAILED_TO_PARSE_JSON_COUNT
                .with_label_values(&["json file too large"])
                .inc();
            return Err(anyhow::anyhow!(format!(
                "JSON parser received file too large: {} bytes, skipping",
                size
            )));
        }

        let op = || {
            async {
                info!(asset_uri = uri, "Sending request for asset_uri");

                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_JSON_REQUEST_RETRY_SECONDS))
                    .build()
                    .context("Failed to build reqwest client")?;

                let response = client
                    .get(uri.trim())
                    .send()
                    .await
                    .context("Failed to get JSON")?;

                let parsed_json = response
                    .json::<Value>()
                    .await
                    .context("Failed to parse JSON")?;

                let raw_image_uri = parsed_json["image"].as_str().map(|s| s.to_string());
                let raw_animation_uri =
                    parsed_json["animation_url"].as_str().map(|s| s.to_string());

                Ok((raw_image_uri, raw_animation_uri, parsed_json))
            }
            .boxed()
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, op).await {
            Ok(result) => {
                SUCCESSFULLY_PARSED_JSON_COUNT.inc();
                Ok(result)
            },
            Err(e) => {
                FAILED_TO_PARSE_JSON_COUNT
                    .with_label_values(&["other"])
                    .inc();
                Err(e)
            },
        }
    }
}
