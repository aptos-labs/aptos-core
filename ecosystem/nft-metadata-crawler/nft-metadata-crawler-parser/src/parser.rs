// Copyright © Aptos Foundation

use crate::{
    db::upsert_uris,
    models::{NFTMetadataCrawlerURIs, NFTMetadataCrawlerURIsQuery},
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use google_cloud_auth::token_source::TokenSource;
use image::{ImageBuffer, ImageFormat};
use nft_metadata_crawler_utils::{
    gcs::{write_image_to_gcs, write_json_to_gcs},
    NFTMetadataCrawlerEntry,
};
use serde_json::Value;
use tracing::{error, info};

// Stuct that represents a parser for a single entry from queue
pub struct Parser<'a> {
    pub entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    format: ImageFormat,
    bucket: String,
    ts: &'a dyn TokenSource,
    force: bool,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
}

impl<'a> Parser<'a> {
    pub fn new(
        entry: NFTMetadataCrawlerEntry,
        bucket: String,
        force: bool,
        ts: &'a dyn TokenSource,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs {
                token_uri: entry.token_uri.clone(),
                raw_image_uri: None,
                raw_animation_uri: None,
                cdn_json_uri: None,
                cdn_image_uri: None,
                cdn_animation_uri: None,
                image_resizer_retry_count: 0,
                json_parser_retry_count: 0,
            },
            entry,
            format: ImageFormat::Jpeg,
            bucket,
            ts,
            force,
            conn,
        }
    }

    pub async fn parse(&mut self) -> anyhow::Result<()> {
        // Deduplication
        if !self.force
            && NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.entry.token_uri.clone(),
                &mut self.conn,
            )
            .unwrap_or(None)
            .is_some()
        {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Skipping URI parse"
            );
            return Ok(());
        }

        // URI Parser
        let json_uri = match Self::parse_uri(self.entry.token_uri.clone()) {
            Ok(u) => u,
            Err(_) => self.entry.token_uri.clone(),
        };

        // JSON Parser
        match self.parse_json(json_uri).await {
            Ok(json) => {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Successfully parsed JSON"
                );

                // Write JSON to GCS
                match write_json_to_gcs(
                    self.ts,
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    json,
                )
                .await
                {
                    Ok(filename) => {
                        // Temporarily hardcode IP for load balancer testing
                        self.model.cdn_json_uri =
                            Some(format!("http://34.160.26.161/{}", filename));
                        info!(
                            last_transaction_version = self.entry.last_transaction_version,
                            "Successfully saved JSON"
                        )
                    },
                    Err(e) => error!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "{}",
                        e.to_string()
                    ),
                }
            },
            Err(e) => {
                self.model.json_parser_retry_count += 1;
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "{}",
                    e.to_string()
                )
            },
        }

        // Save to Postgres
        match upsert_uris(&mut self.conn, self.model.clone()) {
            Ok(_) => info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Successfully upserted JSON URIs"
            ),
            Err(e) => error!(
                last_transaction_version = self.entry.last_transaction_version,
                "{}",
                e.to_string()
            ),
        }

        // URI Parser
        let raw_img_uri = self
            .model
            .raw_image_uri
            .clone()
            .unwrap_or(self.model.token_uri.clone());

        let img_uri = match Self::parse_uri(raw_img_uri.clone()) {
            Ok(u) => u,
            Err(_) => raw_img_uri,
        };

        // Image Optimizer
        match self.optimize_image(img_uri).await {
            Ok(new_img) => {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Successfully optimized image"
                );

                // Write image to GCS
                match write_image_to_gcs(
                    self.ts,
                    self.format,
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    new_img,
                )
                .await
                {
                    Ok(filename) => {
                        self.model.cdn_image_uri =
                            Some(format!("http://34.160.26.161/{}", filename));
                        info!(
                            last_transaction_version = self.entry.last_transaction_version,
                            "Successfully saved image"
                        );
                    },
                    Err(e) => error!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "{}",
                        e.to_string()
                    ),
                }
            },
            Err(e) => {
                self.model.image_resizer_retry_count += 1;
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "{}",
                    e.to_string()
                )
            },
        }

        // Save to Postgres
        match upsert_uris(&mut self.conn, self.model.clone()) {
            Ok(_) => info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Successfully upserted image URIs"
            ),
            Err(e) => error!(
                last_transaction_version = self.entry.last_transaction_version,
                "{}",
                e.to_string()
            ),
        }

        Ok(())
    }

    // Parse URI for IPFS CID and path
    fn parse_uri(_uri: String) -> anyhow::Result<String> {
        todo!();
    }

    // HEAD request to get size of content
    async fn _get_size(&mut self, _url: String) -> anyhow::Result<u32> {
        todo!();
    }

    // Parse JSON for image URI
    async fn parse_json(&mut self, _uri: String) -> anyhow::Result<Value> {
        todo!();
    }

    // Optimize and resize image
    async fn optimize_image(&mut self, _img_uri: String) -> anyhow::Result<Vec<u8>> {
        todo!();
    }

    // Converts image buffer to bytes
    fn _to_bytes(
        &self,
        _image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    ) -> anyhow::Result<Vec<u8>> {
        todo!();
    }
}
