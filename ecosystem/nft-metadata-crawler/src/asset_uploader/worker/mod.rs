// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::worker::config::AssetUploaderWorkerConfig, config::Server,
    utils::constants::MAX_ASSET_UPLOAD_RETRY_SECONDS,
};
use ahash::AHashMap;
use anyhow::Context;
use axum::{
    body::Body,
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use reqwest::{multipart::Form, Client};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use url::Url;

pub mod config;

const MAX_IMAGES_PER_PAGE_CLOUDFLARE_STRING: &str = "10000";

#[derive(Debug, Deserialize)]
struct CloudflareImageListResponseResultImage {
    filename: String,
    id: String,
    meta: Option<AHashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageListResponseResult {
    images: Vec<CloudflareImageListResponseResultImage>,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageListResponse {
    result: Option<CloudflareImageListResponseResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetExistingResponse {
    pub id: String,
}

#[derive(Clone)]
pub struct AssetUploaderWorkerContext {
    config: Arc<AssetUploaderWorkerConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UploadRequest {
    pub url: Url,
}

impl AssetUploaderWorkerContext {
    pub fn new(config: AssetUploaderWorkerConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Uploads an asset to Cloudflare and returns the response
    async fn upload_asset(&self, url: &Url) -> anyhow::Result<impl IntoResponse> {
        let hashed_url = sha256::digest(url.to_string());
        let client = Client::builder()
            .timeout(Duration::from_secs(MAX_ASSET_UPLOAD_RETRY_SECONDS))
            .build()
            .context("Error building reqwest client")?;
        let form = Form::new()
            .text("id", hashed_url.clone())
            .text(
                // Save the asset_uri in the upload metadata to enable retrieval by asset_uri later
                "metadata",
                format!("{{\"asset_uri\": \"{}\"}}", url),
            )
            .text("url", url.to_string());

        info!(
            asset_uri = ?url,
            "[Asset Uploader] Uploading asset to Cloudflare"
        );

        let res = client
            .post(format!(
                "https://api.cloudflare.com/client/v4/accounts/{}/images/v1",
                self.config.cloudflare_account_id
            ))
            .header(
                "Authorization",
                format!("Bearer {}", self.config.cloudflare_auth_key),
            )
            .multipart(form)
            .send()
            .await
            .context("Error sending request to Cloudflare")?;

        reqwest_response_to_axum_response(res).await
    }

    async fn handle_upload(
        Extension(context): Extension<Arc<AssetUploaderWorkerContext>>,
        Json(request): Json<UploadRequest>,
    ) -> impl IntoResponse {
        match context.upload_asset(&request.url).await {
            Ok(res) => {
                let res = res.into_response(); // TODO: How to log response body?
                info!(asset_uri = ?request.url, response = ?res, "[Asset Uploader] Asset uploaded with response");
                res
            },
            Err(e) => {
                error!(asset_uri = ?request.url, error = ?e, "[Asset Uploader] Error uploading asset");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error uploading asset: {}", e),
                )
                    .into_response()
            },
        }
    }

    /// Uploads an asset to Cloudflare and returns the response
    async fn get_by_asset_uri(&self, url: &Url) -> anyhow::Result<Option<String>> {
        let mut page = 1;
        let hashed_url = sha256::digest(url.to_string());
        let client = Client::builder()
            .timeout(Duration::from_secs(MAX_ASSET_UPLOAD_RETRY_SECONDS))
            .build()
            .context("Error building reqwest client")?;
        let mut params = AHashMap::new();
        params.insert(
            "per_page",
            MAX_IMAGES_PER_PAGE_CLOUDFLARE_STRING.to_string(),
        );

        loop {
            info!(
                asset_uri = ?url,
                "[Asset Uploader] Finding asset from Cloudflare"
            );

            params.insert("page", page.to_string());
            let res = client
                .get(format!(
                    "https://api.cloudflare.com/client/v4/accounts/{}/images/v1",
                    self.config.cloudflare_account_id
                ))
                .header(
                    "Authorization",
                    format!("Bearer {}", self.config.cloudflare_auth_key),
                )
                .query(&params)
                .send()
                .await
                .context("Error sending request to Cloudflare")?;

            let body = res.text().await.context("Error reading response body")?;
            let body = serde_json::from_str::<CloudflareImageListResponse>(&body)
                .context("Error parsing response body")?;
            let images = body
                .result
                .context("Error getting result from response body")?
                .images;

            let res = images.par_iter().find_any(|image| {
                // Metadata not guaranteed to exist
                let meta_url = if let Some(meta) = &image.meta {
                    meta.get("asset_uri")
                } else {
                    None
                };

                image.filename == hashed_url || meta_url == Some(&url.to_string())
            });

            if let Some(image) = res {
                return Ok(Some(image.id.clone()));
            }

            if images.len()
                < MAX_IMAGES_PER_PAGE_CLOUDFLARE_STRING
                    .parse::<usize>()
                    .context("Error parsing MAX_IMAGES_PER_PAGE_CLOUDFLARE_STRING")?
            {
                return Ok(None);
            }

            page += 1;
        }
    }

    async fn handle_get_by_asset_uri(
        Extension(context): Extension<Arc<AssetUploaderWorkerContext>>,
        Query(request): Query<UploadRequest>,
    ) -> impl IntoResponse {
        info!(asset_uri = ?request.url, "[Asset Uploader] Retrieving asset by asset_uri");
        match context.get_by_asset_uri(&request.url).await {
            Ok(Some(id)) => {
                info!(asset_uri = ?request.url, id = ?id, "[Asset Uploader] Asset found by asset_uri");
                (StatusCode::OK, Json(GetExistingResponse { id })).into_response()
            },
            Ok(None) => {
                info!(asset_uri = ?request.url, "[Asset Uploader] Asset not found by asset_uri");
                (StatusCode::NOT_FOUND, "Asset not found by asset_uri").into_response()
            },
            Err(e) => {
                error!(asset_uri = ?request.url, error = ?e, "[Asset Uploader] Error retrieving asset by asset_uri");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error retrieving asset by asset_uri {}: {}", request.url, e),
                )
                    .into_response()
            },
        }
    }
}

impl Server for AssetUploaderWorkerContext {
    fn build_router(&self) -> Router {
        Router::new()
            .route("/", post(Self::handle_upload))
            .route("/get_existing", get(Self::handle_get_by_asset_uri))
            .layer(Extension(Arc::new(self.clone())))
    }
}

/// Converts a reqwest response to an axum response
/// Only copies the response status, response body, and Content-Type header
async fn reqwest_response_to_axum_response(
    response: reqwest::Response,
) -> anyhow::Result<impl IntoResponse> {
    let status = response.status();
    let headers = response.headers().clone();

    let body_bytes = response
        .bytes()
        .await
        .context("Error reading response body")?;

    let mut response = axum::http::Response::builder().status(status.as_u16());

    // Set Content-Type header if present
    if let Some(content_type) = headers.get(reqwest::header::CONTENT_TYPE) {
        response = response.header(
            axum::http::header::CONTENT_TYPE,
            content_type
                .to_str()
                .context("Error parsing Content-Type header")?,
        );
    }

    let body = Body::from(body_bytes);
    response.body(body).context("Error building response")
}
