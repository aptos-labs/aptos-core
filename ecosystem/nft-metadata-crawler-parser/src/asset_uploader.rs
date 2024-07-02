// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::NFTMetadataCrawlerConfig,
    models::nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
    utils::{
        constants::{MAX_ASSET_UPLOAD_RETRY_SECONDS, MAX_RETRY_TIME_SECONDS},
        database::upsert_uris,
    },
    Server,
};
use anyhow::Context;
use axum::response::{IntoResponse, Response};
use backoff::{future::retry, ExponentialBackoff};
use bytes::Bytes;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use futures::FutureExt;
use reqwest::{multipart::Form, Client, StatusCode};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AssetUploaderContext {
    pub nft_metadata_crawler_config: Arc<NFTMetadataCrawlerConfig>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponseResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponse {
    result: CloudflareImageUploadResponseResult,
}

impl AssetUploaderContext {
    pub async fn new(
        nft_metadata_crawler_config: Arc<NFTMetadataCrawlerConfig>,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        if nft_metadata_crawler_config.asset_uploader_config.is_none() {
            error!(config = ?nft_metadata_crawler_config, "[Asset Uploader] asset_uploader_config not found");
            panic!();
        }

        Self {
            nft_metadata_crawler_config,
            pool,
        }
    }

    async fn upload_asset(&self, url: &str) -> anyhow::Result<String> {
        let asset_uploader_config = self
            .nft_metadata_crawler_config
            .as_ref()
            .asset_uploader_config
            .as_ref()
            .unwrap();

        let url = url.to_string();
        let hashed_url = sha256::digest(url.clone());
        let op = || {
            async {
                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_ASSET_UPLOAD_RETRY_SECONDS))
                    .build()
                    .context("Failed to build reqwest client")?;

                let form = Form::new()
                    .text("id", format!("tmp/{}", hashed_url)) // Replace with actual metadata
                    .text("url", url.clone());

                info!(
                    asset_uri = url,
                    "[Asset Uploader] Uploading asset to Cloudflare"
                );

                let res = client
                    .post(format!(
                        "https://api.cloudflare.com/client/v4/accounts/{}/images/v1",
                        asset_uploader_config.cloudflare_account_id
                    ))
                    .header(
                        "Authorization",
                        format!("Bearer {}", asset_uploader_config.cloudflare_auth_key),
                    )
                    .multipart(form)
                    .send()
                    .await
                    .context("Failed to upload asset")?;

                let res_text = res.text().await.context("Failed to get response text")?;
                info!(response = ?res_text, "[Asset Uploader] Received response from Cloudflare");

                let res = serde_json::from_str::<CloudflareImageUploadResponse>(&res_text)
                    .context("Failed to parse response to CloudflareImageUploadResponse")?;

                Ok(format!(
                    "{}/{}/{}/{}",
                    asset_uploader_config.cloudflare_image_delivery_prefix,
                    asset_uploader_config.cloudflare_account_hash,
                    res.result.id,
                    asset_uploader_config.cloudflare_default_variant,
                ))
            }
            .boxed()
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, op).await {
            Ok(result) => Ok(result),
            Err(e) => Err(e),
        }
    }
}

#[async_trait::async_trait]
impl Server for AssetUploaderContext {
    /// Handles calling parser for the root endpoint
    async fn handle_request(self: Arc<Self>, msg: Bytes) -> Response {
        let url = String::from_utf8_lossy(&msg).to_string();
        match self.upload_asset(&url).await {
            Ok(cdn_url) => {
                info!(
                    asset_uri = url,
                    cdn_uri = cdn_url,
                    "[Asset Uploader] Writing to Postgres"
                );
                let mut model = NFTMetadataCrawlerURIs::new(&url);
                model.set_cdn_image_uri(Some(cdn_url));

                let mut conn = self.pool.get().unwrap();
                upsert_uris(&mut conn, &model, -1).unwrap_or_else(|e| {
                    error!(error=?e, asset_uri = url, "[Asset Uploader] Commit to Postgres failed");
                    panic!();
                });

                StatusCode::OK.into_response()
            },
            Err(e) => {
                warn!(error = ?e, asset_uri = url, "[Asset Uploader] Failed to upload asset");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            },
        }
    }
}
