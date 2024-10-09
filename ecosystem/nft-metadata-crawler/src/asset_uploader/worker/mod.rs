// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::worker::config::AssetUploaderWorkerConfig, config::Server,
    utils::constants::MAX_ASSET_UPLOAD_RETRY_SECONDS,
};
use anyhow::Context;
use axum::{
    body::Body, http::StatusCode, response::IntoResponse, routing::post, Extension, Json, Router,
};
use reqwest::{multipart::Form, Client};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use url::Url;

pub mod config;

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
            .text("id", hashed_url.clone()) // Replace with actual metadata
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
}

impl Server for AssetUploaderWorkerContext {
    fn build_router(&self) -> Router {
        Router::new()
            .route("/", post(Self::handle_upload))
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
