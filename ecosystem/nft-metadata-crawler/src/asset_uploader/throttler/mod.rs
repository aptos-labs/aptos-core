// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::worker::{GetExistingResponse, UploadRequest},
    config::Server,
    models::{
        asset_uploader_request_statuses::AssetUploaderRequestStatuses,
        asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery,
        parsed_asset_uris::ParsedAssetUris, parsed_asset_uris_query::ParsedAssetUrisQuery,
    },
    schema::{self},
    utils::database::upsert_uris,
};
use ahash::{AHashMap, AHashSet};
use anyhow::Context;
use axum::{http::StatusCode as AxumStatusCode, response::IntoResponse, routing::post, Extension};
use config::AssetUploaderThrottlerConfig;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    upsert::excluded,
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use reqwest::{Client, StatusCode as ReqwestStatusCode};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    fmt::Display,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{Mutex, Notify};
use tracing::{debug, error, info, warn};
use url::Url;

pub mod config;

const FIVE_MINUTES: Duration = Duration::from_secs(60 * 5);

// Structs below are for accessing relevant data in a typed way for Cloudflare API calls
#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponseResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponseError {
    code: i64,
    message: String,
}

impl Display for CloudflareImageUploadResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponse {
    errors: Vec<CloudflareImageUploadResponseError>,
    result: Option<CloudflareImageUploadResponseResult>,
}

#[derive(Clone, Debug)]
pub struct UploadQueue {
    asset_queue: BTreeSet<AssetUploaderRequestStatuses>,
    in_progress_assets: AHashSet<AssetUploaderRequestStatuses>,
}

#[derive(Clone)]
pub struct AssetUploaderThrottlerContext {
    config: AssetUploaderThrottlerConfig,
    pool: Pool<ConnectionManager<PgConnection>>,
    upload_queue: Arc<Mutex<UploadQueue>>,
    inserted_notify: Arc<Notify>,
    is_rate_limited: Arc<AtomicBool>,
    rate_limit_over_notify: Arc<Notify>,
    client: Arc<Client>,
}

impl AssetUploaderThrottlerContext {
    pub fn new(
        config: AssetUploaderThrottlerConfig,
        pool: Pool<ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            config,
            pool,
            upload_queue: Arc::new(Mutex::new(UploadQueue {
                asset_queue: BTreeSet::new(),
                in_progress_assets: AHashSet::new(),
            })),
            inserted_notify: Arc::new(Notify::new()),
            is_rate_limited: Arc::new(AtomicBool::new(false)),
            rate_limit_over_notify: Arc::new(Notify::new()),
            client: Arc::new(Client::new()),
        }
    }

    async fn upload_asset(
        &self,
        asset: AssetUploaderRequestStatuses,
    ) -> anyhow::Result<AssetUploaderRequestStatuses> {
        // Make a request to the worker to upload the asset
        info!(asset_uri = ?asset.asset_uri, "Requesting worker to upload asset");
        let res = self
            .client
            .post(self.config.asset_uploader_worker_uri.clone())
            .json(&UploadRequest {
                url: Url::parse(&asset.asset_uri)?,
            })
            .send()
            .await
            .context("Error sending upload request to worker")?;

        let status = res.status();
        let body = res.text().await?;
        let body = serde_json::from_str::<CloudflareImageUploadResponse>(&body)?;

        // Update the request in Postgres with the response
        let mut asset = asset;
        asset.status_code = status.as_u16() as i64;
        if status == ReqwestStatusCode::OK {
            let cdn_image_uri = Some(format!(
                "{}/{}/{}/{}",
                self.config.cloudflare_image_delivery_prefix,
                self.config.cloudflare_account_hash,
                body.result.context("Result not found")?.id,
                self.config.cloudflare_default_variant,
            ));

            asset.cdn_image_uri.clone_from(&cdn_image_uri);

            // Update the asset URI in the parsed_asset_uris table
            let mut parsed_asset_uri = ParsedAssetUris::new(&asset.asset_uri);
            parsed_asset_uri.set_cdn_image_uri(cdn_image_uri);
            upsert_uris(&mut self.pool.get()?, &parsed_asset_uri, 0)?;
        } else {
            asset.num_failures += 1;
            asset.error_messages = Some(
                body.errors
                    .iter()
                    .map(|err| Some(err.to_string()))
                    .collect::<Vec<_>>(),
            );
        }

        self.update_request_status(&asset)?;
        Ok(asset)
    }

    async fn get_from_cloudflare(
        &self,
        asset: AssetUploaderRequestStatuses,
    ) -> anyhow::Result<AssetUploaderRequestStatuses> {
        // Make a request to the worker to lookup the asset
        info!(asset_uri = ?asset.asset_uri, "Requesting worker to lookup asset");
        let mut asset_uploader_worker_uri = Url::parse(&self.config.asset_uploader_worker_uri)?;
        asset_uploader_worker_uri.set_path("get_existing");
        let res = self
            .client
            .get(asset_uploader_worker_uri)
            .query(&AHashMap::from_iter(vec![(
                "url".to_string(),
                asset.asset_uri.clone(),
            )]))
            .send()
            .await
            .context("Error sending upload request to worker")?;

        let status = res.status();
        let body = res.text().await?;
        let body = serde_json::from_str::<GetExistingResponse>(&body)?;

        // Update the request in Postgres with the response
        let mut asset = asset;
        asset.status_code = status.as_u16() as i64;
        if status == ReqwestStatusCode::OK {
            let cdn_image_uri = Some(format!(
                "{}/{}/{}/{}",
                self.config.cloudflare_image_delivery_prefix,
                self.config.cloudflare_account_hash,
                body.id,
                self.config.cloudflare_default_variant,
            ));

            asset.cdn_image_uri.clone_from(&cdn_image_uri);

            // Update the asset URI in the parsed_asset_uris table
            let mut parsed_asset_uri = ParsedAssetUris::new(&asset.asset_uri);
            parsed_asset_uri.set_cdn_image_uri(cdn_image_uri);
            upsert_uris(&mut self.pool.get()?, &parsed_asset_uri, 0)?;
        } else {
            asset.num_failures += 1;
            asset.error_messages = Some(vec![Some("Asset not found in Cloudflare".to_string())]);
        }

        self.update_request_status(&asset)?;
        Ok(asset)
    }

    async fn handle_upload_assets(&self) {
        let self_arc = Arc::new(self.clone());
        loop {
            // Wait until notified if rate limited
            while self.is_rate_limited.load(Ordering::Relaxed) {
                self.rate_limit_over_notify.notified().await;
                self.is_rate_limited.store(false, Ordering::Relaxed);
            }

            // Wait until notified if queue is empty
            while self.upload_queue.lock().await.asset_queue.is_empty() {
                self.inserted_notify.notified().await;
            }

            // Pop the first asset from the queue and add it to the in-progress set
            let mut upload_queue = self.upload_queue.lock().await;
            // Should be safe to unwrap because we checked if the queue is empty, but log in case
            let Some(asset) = upload_queue.asset_queue.pop_first() else {
                warn!(
                    queue = ?upload_queue,
                    "Asset queue is empty, despite being notified"
                );
                continue;
            };
            upload_queue.in_progress_assets.insert(asset.clone());
            drop(upload_queue);

            // Upload the asset in a separate task
            // If successful, remove the asset from the in-progress set and continue to next asset
            // If rate limited, sleep for 5 minutes then notify
            // If unsuccessful due to conflict, attempt to lookup the asset in Cloudflare
            // If unsuccessful for other reason, add the asset back to the queue
            let self_clone = self_arc.clone();
            tokio::spawn(async move {
                // Handle upload depending on previous attempt status.
                // If previous attempt resulted in a 409, the asset likely already exists, so we call a different endpoint on the worker to perform the lookup.
                let upload_res = match ReqwestStatusCode::from_u16(asset.status_code as u16)? {
                    ReqwestStatusCode::CONFLICT => {
                        self_clone.get_from_cloudflare(asset.clone()).await
                    },
                    _ => self_clone.upload_asset(asset.clone()).await,
                };

                let mut upload_queue = self_clone.upload_queue.lock().await;
                match upload_res {
                    Ok(asset) => {
                        let mut asset = asset;
                        match ReqwestStatusCode::from_u16(asset.status_code as u16)? {
                            ReqwestStatusCode::OK => {
                                // If success, remove asset from in-progress set and end early
                                upload_queue.in_progress_assets.remove(&asset);
                                anyhow::Ok(())
                            },
                            ReqwestStatusCode::TOO_MANY_REQUESTS => {
                                // If rate limited, sleep for 5 minutes then notify
                                self_clone.is_rate_limited.store(true, Ordering::Relaxed);
                                tokio::time::sleep(FIVE_MINUTES).await;
                                self_clone.rate_limit_over_notify.notify_one();
                                Ok(())
                            },
                            ReqwestStatusCode::CONFLICT => {
                                // If conflict, attempt to get cdn_image_uri from parsed_asset_uris table
                                if let Some(parsed_asset_uri) =
                                    ParsedAssetUrisQuery::get_by_asset_uri(
                                        &mut self_clone.pool.get()?,
                                        &asset.asset_uri,
                                    )
                                {
                                    // If cdn_image_uri found, update asset and request status
                                    if let Some(cdn_image_uri) = parsed_asset_uri.cdn_image_uri {
                                        asset.cdn_image_uri = Some(cdn_image_uri);
                                        self_clone.update_request_status(&asset)?;
                                        return Ok(());
                                    }
                                }

                                // If cdn_image_uri still not found and num_failures < 3, add asset back to queue.
                                if asset.cdn_image_uri.is_none() && asset.num_failures < 3 {
                                    self_clone.update_request_status(&asset)?;
                                    upload_queue.asset_queue.insert(asset);
                                    self_clone.inserted_notify.notify_one();
                                    return Ok(());
                                }

                                // Remove asset from in-progress set and end early.
                                // No point in retrying more than 3 times because the asset already exists and could not be found in Postgrs or Cloudflare.
                                upload_queue.in_progress_assets.remove(&asset);
                                Ok(())
                            },
                            _ => Ok(()),
                        }
                    },
                    Err(e) => {
                        error!(error = ?e, asset_uri = asset.asset_uri, "[Asset Uploader Throttler] Error uploading asset");
                        upload_queue.asset_queue.insert(asset);
                        Ok(())
                    },
                }
            });
        }
    }

    async fn update_queue(&self) -> anyhow::Result<usize> {
        use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

        let query = asset_uploader_request_statuses
            .filter(status_code.ne(ReqwestStatusCode::OK.as_u16() as i64))
            .order_by(inserted_at.asc())
            .limit(self.config.poll_rows_limit as i64);

        let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
        debug!("Executing Query: {}", debug_query);
        let rows: Vec<AssetUploaderRequestStatusesQuery> = query.load(&mut self.pool.get()?)?;

        let mut num_queued = 0;
        for row in rows {
            let row: AssetUploaderRequestStatuses = (&row).into();
            let upload_queue = &mut self.upload_queue.lock().await;
            if !upload_queue.in_progress_assets.contains(&row) {
                upload_queue.asset_queue.insert(row);
                num_queued += 1;
            }
        }

        Ok(num_queued)
    }

    async fn start_update_loop(&self) -> anyhow::Result<()> {
        let poll_interval_seconds = Duration::from_secs(self.config.poll_interval_seconds);
        loop {
            let num_queued = self.update_queue().await?;
            if num_queued > 0 {
                self.inserted_notify.notify_one();
            }

            tokio::time::sleep(poll_interval_seconds).await;
        }
    }

    async fn handle_update_queue(Extension(context): Extension<Arc<Self>>) -> impl IntoResponse {
        match context.update_queue().await {
            Ok(_) => AxumStatusCode::OK,
            Err(e) => {
                error!(error = ?e, "[Asset Uploader Throttler] Error updating queue");
                AxumStatusCode::INTERNAL_SERVER_ERROR
            },
        }
    }

    fn update_request_status(&self, asset: &AssetUploaderRequestStatuses) -> anyhow::Result<()> {
        use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

        let query = diesel::insert_into(asset_uploader_request_statuses)
            .values(asset)
            .on_conflict((idempotency_key, application_id, asset_uri))
            .do_update()
            .set((
                status_code.eq(excluded(status_code)),
                error_messages.eq(excluded(error_messages)),
                cdn_image_uri.eq(excluded(cdn_image_uri)),
                num_failures.eq(excluded(num_failures)),
                inserted_at.eq(excluded(inserted_at)),
            ));

        let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
        debug!("Executing Query: {}", debug_query);
        query.execute(&mut self.pool.get()?)?;
        Ok(())
    }
}

impl Server for AssetUploaderThrottlerContext {
    fn build_router(&self) -> axum::Router {
        let self_arc = Arc::new(self.clone());

        let self_arc_clone = self_arc.clone();
        tokio::spawn(async move {
            self_arc_clone.handle_upload_assets().await;
        });

        let self_arc_clone = self_arc.clone();
        tokio::spawn(async move {
            self_arc_clone.start_update_loop().await?;
            anyhow::Ok(())
        });

        axum::Router::new()
            .route("/update_queue", post(Self::handle_update_queue))
            .layer(Extension(self_arc.clone()))
    }
}
