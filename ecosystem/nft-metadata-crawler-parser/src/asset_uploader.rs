// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::ParserConfig,
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
use std::{sync::Arc, time::Duration};
use tracing::error;

#[derive(Clone)]
pub struct AssetUploaderContext {
    pub parser_config: Arc<ParserConfig>,
    pub pool: Pool<ConnectionManager<PgConnection>>,
}

impl AssetUploaderContext {
    pub async fn new(
        parser_config: Arc<ParserConfig>,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        if parser_config.cloudflare_auth_key.is_none() {
            error!("Cloudflare auth key not found in config");
            panic!();
        }

        Self {
            parser_config,
            pool,
        }
    }

    async fn upload_asset(&self, url: &str) -> anyhow::Result<String> {
        let url = url.to_string();
        let op = || {
            async {
                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_ASSET_UPLOAD_RETRY_SECONDS))
                    .build()
                    .context("Failed to build reqwest client")?;

                // TODO
                let form = Form::new()
                    .text("file", "") // Replace with actual file path or content
                    .text("metadata", "") // Replace with actual metadata
                    .text("url", url.clone());

                client
                    .post("https://api.cloudflare.com/client/v4/accounts/{account_id}/images/v1")
                    .header(
                        "Authorization",
                        format!(
                            "Bearer {}",
                            self.parser_config
                                .cloudflare_auth_key
                                .as_ref()
                                .unwrap()
                                .clone()
                        ),
                    )
                    .multipart(form)
                    .send()
                    .await
                    .context("Failed to upload asset")?;

                Ok(
                    "https://imagedelivery.net/<ACCOUNT_HASH>/<IMAGE_ID>/w=400,sharpen=3"
                        .to_string(),
                )
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
                let mut model = NFTMetadataCrawlerURIs::new(&url);
                model.set_cdn_image_uri(Some(cdn_url));

                let mut conn = self.pool.get().unwrap();
                upsert_uris(&mut conn, &model, -1).unwrap_or_else(|e| {
                    error!(error=?e,"Commit to Postgres failed");
                    panic!();
                });

                StatusCode::OK.into_response()
            },
            Err(e) => {
                error!(error = ?e, "Failed to upload asset");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            },
        }
    }
}
