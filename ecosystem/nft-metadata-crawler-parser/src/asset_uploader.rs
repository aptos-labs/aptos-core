// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{AssetUploaderConfig, Server},
    models::nft_metadata_crawler_uris::NFTMetadataCrawlerURIs,
    utils::{
        constants::{MAX_ASSET_UPLOAD_RETRY_SECONDS, MAX_RETRY_TIME_SECONDS},
        database::upsert_uris,
    },
};
use anyhow::Context;
use axum::{routing::post, Extension, Json, Router};
use backoff::{future::retry, ExponentialBackoff};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    PgConnection,
};
use futures::{future::try_join_all, FutureExt};
use reqwest::{multipart::Form, Client};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tracing::{info, warn};
use url::Url;

#[derive(Clone)]
pub struct AssetUploaderContext {
    pub asset_uploader_config: Arc<AssetUploaderConfig>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

// Structs below are for accessing relevant data in a typed way for Cloudflare API calls
#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponseResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponse {
    result: CloudflareImageUploadResponseResult,
}

#[derive(Debug, Deserialize)]
struct AssetUploaderRequest {
    urls: Vec<Url>,
}

#[derive(Debug, Serialize)]
struct AssetUploaderResponse {
    successes: Vec<String>,
    failures: Vec<String>,
}

impl AssetUploaderContext {
    pub fn new(
        asset_uploader_config: AssetUploaderConfig,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            asset_uploader_config: Arc::new(asset_uploader_config),
            pool,
        }
    }

    /// Uploads an asset to Cloudflare and returns the CDN URL used to access it
    /// The CDN URL uses the default variant specified in the config
    async fn upload_asset(&self, url: Url) -> anyhow::Result<String> {
        let hashed_url = sha256::digest(url.to_string());
        let op = || {
            async {
                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_ASSET_UPLOAD_RETRY_SECONDS))
                    .build()
                    .context("Failed to build reqwest client")?;

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
                        self.asset_uploader_config.cloudflare_account_id
                    ))
                    .header(
                        "Authorization",
                        format!("Bearer {}", self.asset_uploader_config.cloudflare_auth_key),
                    )
                    .multipart(form)
                    .send()
                    .await
                    .context("Failed to send request to Cloudflare API")?;

                let res_text = res.text().await.context("Failed to get response text")?;
                info!(response = ?res_text, "[Asset Uploader] Received response from Cloudflare");

                let res = serde_json::from_str::<CloudflareImageUploadResponse>(&res_text)
                    .context("Failed to parse response to CloudflareImageUploadResponse")?;

                Ok(format!(
                    "{}/{}/{}/{}",
                    self.asset_uploader_config.cloudflare_image_delivery_prefix,
                    self.asset_uploader_config.cloudflare_account_hash,
                    res.result.id,
                    self.asset_uploader_config.cloudflare_default_variant,
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

    async fn handle_urls(
        Extension(context): Extension<Arc<Self>>,
        Json(urls): Json<AssetUploaderRequest>,
    ) -> Json<AssetUploaderResponse> {
        // Spawn a task for each URL
        let mut tasks = Vec::with_capacity(urls.urls.len());
        let self_clone = context.clone();
        for url in urls.urls.clone() {
            let self_clone = self_clone.clone();
            tasks.push(tokio::spawn(async move {
                match self_clone.upload_asset(url.clone()).await {
                    Ok(cdn_url) => {
                        info!(
                            asset_uri = ?url,
                            cdn_uri = cdn_url,
                            "[Asset Uploader] Writing to Postgres"
                        );
                        let mut model = NFTMetadataCrawlerURIs::new(url.as_ref());
                        model.set_cdn_image_uri(Some(cdn_url.clone()));

                        let mut conn = self_clone.pool.get().context("Failed to get connection")?;
                        upsert_uris(&mut conn, &model, -1).context("Failed to upsert URIs")?;

                        Ok(cdn_url)
                    },
                    Err(e) => {
                        warn!(error = ?e, asset_uri = ?url, "[Asset Uploader] Failed to upload asset");
                        Err(e)
                    },
                }
            }));
        }

        // Wait for all tasks to finish
        match try_join_all(tasks).await {
            Ok(uris) => {
                let mut successes = Vec::new();
                let mut failures = Vec::new();

                for (i, uri) in uris.iter().enumerate() {
                    match uri {
                        Ok(uri) => successes.push(uri.clone()),
                        Err(_) => failures.push(urls.urls[i].to_string()),
                    }
                }

                info!(successes = ?successes, failures = ?failures, "[Asset Uploader] Uploaded assets");
                Json(AssetUploaderResponse {
                    successes,
                    failures,
                })
            },
            Err(_) => Json(AssetUploaderResponse {
                successes: vec![],
                failures: urls.urls.iter().map(|url| url.to_string()).collect(),
            }),
        }
    }
}

impl Server for AssetUploaderContext {
    fn build_router(&self) -> Router {
        let self_arc = Arc::new(self.clone());
        Router::new()
            .route("/", post(Self::handle_urls))
            .layer(Extension(self_arc.clone()))
    }
}
