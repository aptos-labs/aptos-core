// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{asset_uploader::api::get_status::get_status, config::Server};
use ahash::AHashMap;
use axum::{
    extract::Path,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json,
};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::error;
use upload_batch::upload_batch;
use url::Url;

mod get_status;
mod upload_batch;

#[derive(Clone)]
pub struct AssetUploaderApiContext {
    pool: Pool<ConnectionManager<PgConnection>>,
}

#[derive(Debug, Deserialize)]
struct BatchUploadRequest {
    application_id: String,
    urls: Vec<Url>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum BatchUploadResponse {
    Success { request_id: String },
    Error { error: String },
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum GetStatusResponseSuccess {
    Success {
        status_code: Option<u16>,
        cdn_image_uri: String,
    },
    Error {
        status_code: Option<u16>,
        error_message: Option<String>,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
enum GetStatusResponse {
    Success {
        request_id: String,
        urls: AHashMap<String, GetStatusResponseSuccess>,
    },
    Error {
        error: String,
    },
}

impl AssetUploaderApiContext {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    async fn handle_upload_batch(
        Extension(context): Extension<Arc<AssetUploaderApiContext>>,
        Json(request): Json<BatchUploadRequest>,
    ) -> impl IntoResponse {
        match upload_batch(context.pool.clone(), &request) {
            Ok(request_id) => (
                StatusCode::OK,
                Json(BatchUploadResponse::Success { request_id }),
            ),
            Err(e) => {
                error!(error = ?e, "Error uploading asset");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(BatchUploadResponse::Error {
                        error: format!("Error uploading asset: {}", e),
                    }),
                )
            },
        }
    }

    async fn handle_get_status(
        Extension(context): Extension<Arc<AssetUploaderApiContext>>,
        Path(request_id): Path<String>, // Extracts request_id from the URL
    ) -> impl IntoResponse {
        match get_status(context.pool.clone(), &request_id) {
            Ok(statuses) => (
                StatusCode::OK,
                Json(GetStatusResponse::Success {
                    request_id,
                    urls: statuses,
                }),
            ),
            Err(e) => {
                error!(error = ?e, "Error getting status");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(GetStatusResponse::Error {
                        error: format!("Error getting status: {}", e),
                    }),
                )
            },
        }
    }
}

impl Server for AssetUploaderApiContext {
    fn build_router(&self) -> axum::Router {
        let self_arc = Arc::new(self.clone());
        axum::Router::new()
            .route("/upload", post(Self::handle_upload_batch))
            .route("/status/:request_id", get(Self::handle_get_status))
            .layer(Extension(self_arc.clone()))
    }
}
