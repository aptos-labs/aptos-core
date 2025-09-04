// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use reqwest::{header, Client};
use std::time::Duration;
use utils::constants::MAX_HEAD_REQUEST_RETRY_SECONDS;

pub mod asset_uploader;
pub mod config;
pub mod models;
pub mod parser;
pub mod schema;
pub mod utils;

/// HEAD request to get MIME type and size of content
pub async fn get_uri_metadata(url: &str) -> anyhow::Result<(String, u32)> {
    let client = Client::builder()
        .timeout(Duration::from_secs(MAX_HEAD_REQUEST_RETRY_SECONDS))
        .build()
        .context("Failed to build reqwest client")?;
    let request = client.head(url.trim());
    let response = request.send().await?;
    let headers = response.headers();

    let mime_type = headers
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap_or("text/plain"))
        .unwrap_or("text/plain")
        .to_string();
    let size = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(0);

    Ok((mime_type, size))
}
