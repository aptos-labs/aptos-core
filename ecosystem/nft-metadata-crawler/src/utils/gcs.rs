// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{
    constants::MAX_RETRY_TIME_SECONDS,
    counters::{
        FAILED_TO_UPLOAD_TO_GCS_COUNT, GCS_UPLOAD_INVOCATION_COUNT,
        SUCCESSFULLY_UPLOADED_TO_GCS_COUNT,
    },
};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use google_cloud_storage::{
    client::Client,
    http::objects::upload::{Media, UploadObjectRequest, UploadType},
};
use image::ImageFormat;
use serde_json::Value;
use std::time::Duration;

/// Writes JSON Value to GCS
pub async fn write_json_to_gcs(
    bucket: &str,
    uri: &str,
    json: &Value,
    client: &Client,
) -> anyhow::Result<String> {
    GCS_UPLOAD_INVOCATION_COUNT.inc();
    let hashed_uri = sha256::digest(uri);
    let filename = format!("cdn/{}.json", hashed_uri);
    let json_string = json.to_string();
    let json_bytes = json_string.into_bytes();

    let upload_type = UploadType::Simple(Media {
        name: filename.clone().into(),
        content_type: "application/json".into(),
        content_length: Some(json_bytes.len() as u64),
    });

    let op = || {
        async {
            Ok(client
                .upload_object(
                    &UploadObjectRequest {
                        bucket: bucket.to_string(),
                        ..Default::default()
                    },
                    json_bytes.clone(),
                    &upload_type,
                )
                .await
                .context("Error uploading JSON to GCS")?)
        }
        .boxed()
    };

    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
        ..Default::default()
    };

    match retry(backoff, op).await {
        Ok(_) => {
            SUCCESSFULLY_UPLOADED_TO_GCS_COUNT.inc();
            Ok(filename)
        },
        Err(e) => {
            FAILED_TO_UPLOAD_TO_GCS_COUNT.inc();
            Err(e)
        },
    }
}

/// Infers file type and writes image to GCS
pub async fn write_image_to_gcs(
    img_format: ImageFormat,
    bucket: &str,
    uri: &str,
    buffer: Vec<u8>,
    client: &Client,
) -> anyhow::Result<String> {
    GCS_UPLOAD_INVOCATION_COUNT.inc();
    let hashed_uri = sha256::digest(uri);
    let extension = match img_format {
        ImageFormat::Gif | ImageFormat::Avif | ImageFormat::Png => img_format
            .extensions_str()
            .last()
            .expect("ImageFormat should have at least one extension")
            .to_string(),
        _ => "jpeg".to_string(),
    };

    let filename = format!("cdn/{}.{}", hashed_uri, extension);
    let upload_type = UploadType::Simple(Media {
        name: filename.clone().into(),
        content_type: format!("image/{}", extension).into(),
        content_length: Some(buffer.len() as u64),
    });

    let op = || {
        async {
            Ok(client
                .upload_object(
                    &UploadObjectRequest {
                        bucket: bucket.to_string(),
                        ..Default::default()
                    },
                    buffer.clone(),
                    &upload_type,
                )
                .await
                .context("Error uploading image to GCS")?)
        }
        .boxed()
    };

    let backoff = ExponentialBackoff {
        max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
        ..Default::default()
    };

    match retry(backoff, op).await {
        Ok(_) => {
            SUCCESSFULLY_UPLOADED_TO_GCS_COUNT.inc();
            Ok(filename)
        },
        Err(e) => {
            FAILED_TO_UPLOAD_TO_GCS_COUNT.inc();
            Err(e)
        },
    }
}
