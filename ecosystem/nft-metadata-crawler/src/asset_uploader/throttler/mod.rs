// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::worker::UploadRequest,
    config::Server,
    models::{
        asset_uploader_request_statuses::AssetUploaderRequestStatuses,
        asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery,
    },
    schema,
};
use ahash::AHashSet;
use anyhow::Context;
use axum::{http::StatusCode as AxumStatusCode, response::IntoResponse, routing::post, Extension};
use config::AssetUploaderThrottlerConfig;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    upsert::excluded,
    ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl,
};
use parking_lot::Mutex;
use reqwest::{Client, StatusCode as ReqwestStatusCode};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::Notify;
use tracing::{debug, error};
use url::Url;

pub mod config;

const FIVE_MINUTES: Duration = Duration::from_secs(60 * 5);

// Structs below are for accessing relevant data in a typed way for Cloudflare API calls
#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponseResult {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CloudflareImageUploadResponse {
    errors: Vec<String>,
    result: CloudflareImageUploadResponseResult,
}

#[derive(Clone)]
pub struct AssetUploaderThrottlerContext {
    config: AssetUploaderThrottlerConfig,
    pool: Pool<ConnectionManager<PgConnection>>,
    asset_queue: Arc<Mutex<BTreeSet<AssetUploaderRequestStatusesQuery>>>,
    in_progress_assets: Arc<Mutex<AHashSet<AssetUploaderRequestStatusesQuery>>>,
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
            asset_queue: Arc::new(Mutex::new(BTreeSet::new())),
            in_progress_assets: Arc::new(Mutex::new(AHashSet::new())),
            inserted_notify: Arc::new(Notify::new()),
            is_rate_limited: Arc::new(AtomicBool::new(false)),
            rate_limit_over_notify: Arc::new(Notify::new()),
            client: Arc::new(Client::new()),
        }
    }

    async fn upload_asset(
        &self,
        asset: &AssetUploaderRequestStatusesQuery,
    ) -> anyhow::Result<ReqwestStatusCode> {
        use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

        // Make a request to the worker to upload the asset
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
        let mut asset: AssetUploaderRequestStatuses = asset.into();
        asset.status_code = status.as_u16() as i64;
        if status == ReqwestStatusCode::OK {
            asset.cdn_image_uri = Some(format!(
                "{}/{}/{}/{}",
                self.config.cloudflare_image_delivery_prefix,
                self.config.cloudflare_account_hash,
                body.result.id,
                self.config.cloudflare_default_variant,
            ));
        } else {
            asset.num_failures += 1;
            asset.error_message = Some(body.errors.join(", "));
        }

        let query = diesel::insert_into(asset_uploader_request_statuses)
            .values(asset)
            .on_conflict((request_id, asset_uri))
            .do_update()
            .set((
                status_code.eq(excluded(status_code)),
                error_message.eq(excluded(error_message)),
                cdn_image_uri.eq(excluded(cdn_image_uri)),
                num_failures.eq(excluded(num_failures)),
                inserted_at.eq(excluded(inserted_at)),
            ));

        let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
        debug!("Executing Query: {}", debug_query);
        query.execute(&mut self.pool.get()?)?;

        Ok(status)
    }

    async fn handle_upload_assets(&self) {
        let self_arc = Arc::new(self.clone());
        loop {
            // Wait until notified if rate limited
            if self.is_rate_limited.load(Ordering::Relaxed) {
                self.rate_limit_over_notify.notified().await;
                self.is_rate_limited.store(false, Ordering::Relaxed);
            }

            // Wait until notified if queue is empty
            let is_empty = self.asset_queue.lock().is_empty();
            if is_empty {
                self.inserted_notify.notified().await;
            }

            // Pop the first asset from the queue and add it to the in-progress set
            let mut queue = self.asset_queue.lock();
            let asset = queue.pop_first().unwrap(); // Safe to unwrap because we checked if the queue is empty
            let mut in_progress = self.in_progress_assets.lock();
            in_progress.insert(asset.clone());
            drop(in_progress);
            drop(queue);

            // Upload the asset in a separate task
            // If successful, remove the asset from the in-progress set and continue to next asset
            // If unsuccessful, add the asset back to the queue
            let self_clone = self_arc.clone();
            tokio::spawn(async move {
                if let Ok(res) = self_clone.upload_asset(&asset).await {
                    if res.is_success() {
                        let mut in_progress = self_clone.in_progress_assets.lock();
                        in_progress.remove(&asset);
                    } else {
                        // If rate limited, sleep for 5 minutes then notify
                        if res == ReqwestStatusCode::TOO_MANY_REQUESTS {
                            self_clone.is_rate_limited.store(true, Ordering::Relaxed);
                            tokio::time::sleep(FIVE_MINUTES).await;
                            self_clone.rate_limit_over_notify.notify_one();
                        }

                        let mut queue = self_clone.asset_queue.lock();
                        queue.insert(asset);
                    };
                } else {
                    error!(asset_uri = ?asset.asset_uri, "[Asset Uploader Throttler] Error uploading asset");
                    let mut queue = self_clone.asset_queue.lock();
                    queue.insert(asset);
                }
            });
        }
    }

    async fn update_queue(&self) -> anyhow::Result<()> {
        use schema::nft_metadata_crawler::asset_uploader_request_statuses::dsl::*;

        let query = asset_uploader_request_statuses
            .filter(status_code.ne(ReqwestStatusCode::OK.as_u16() as i64))
            .order_by(inserted_at.asc())
            .limit(self.config.poll_rows_limit as i64);

        let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
        debug!("Executing Query: {}", debug_query);
        let rows = query.load(&mut self.pool.get()?)?;

        let mut queue = self.asset_queue.lock();
        let in_progress = self.in_progress_assets.lock();
        for row in rows {
            if !queue.contains(&row) && !in_progress.contains(&row) {
                queue.insert(row);
            }
        }

        Ok(())
    }

    async fn start_update_loop(&self) {
        let poll_interval_seconds = Duration::from_secs(self.config.poll_interval_seconds);
        loop {
            if let Err(e) = self.update_queue().await {
                error!(error = ?e, "[Asset Uploader Throttler] Error updating queue");
            }
            self.inserted_notify.notify_one();
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
            self_arc_clone.start_update_loop().await;
        });

        axum::Router::new()
            .route("/update_queue", post(Self::handle_update_queue))
            .layer(Extension(self_arc.clone()))
    }
}
