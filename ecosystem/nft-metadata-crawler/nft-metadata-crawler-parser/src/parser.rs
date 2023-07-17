// Copyright © Aptos Foundation

use crate::{
    db::upsert_uris,
    models::{NFTMetadataCrawlerURIs, NFTMetadataCrawlerURIsQuery},
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use image::{ImageBuffer, ImageFormat};
use nft_metadata_crawler_utils::{
    gcs::{write_image_to_gcs, write_json_to_gcs},
    NFTMetadataCrawlerEntry,
};
use serde_json::Value;
use tracing::{error, info};

// Stuct that represents a parser for a single entry from queue
pub struct Parser {
    pub entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    bucket: String,
    token: String,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    cdn_prefix: String,
}

impl Parser {
    pub fn new(
        entry: NFTMetadataCrawlerEntry,
        bucket: String,
        token: String,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        cdn_prefix: String,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs::new(entry.token_uri.clone()),
            entry,
            bucket,
            token,
            conn,
            cdn_prefix,
        }
    }

    // Main parsing flow
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        // Deduplication
        if !self.entry.force
            && NFTMetadataCrawlerURIsQuery::get_by_token_uri(
                self.entry.token_uri.clone(),
                &mut self.conn,
            )
            .expect("Unable to get URIs")
            .is_some()
        {
            info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Skipping URI parse"
            );
            return Ok(());
        }

        let json_uri = self.handle_uri_parser_jsons();
        self.handle_json_parser(json_uri).await;
        self.save_to_postgres().await;
        let (img_uri, animation_uri) = self.handle_uri_parser_images();
        self.handle_image_optimizer(img_uri, animation_uri).await;
        self.save_to_postgres().await;

        Ok(())
    }

    // Calls and handles error for JSON parser
    async fn handle_json_parser(&mut self, json_uri: String) {
        match self.parse_json(json_uri).await {
            Ok(json) => {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Successfully parsed JSON"
                );

                // Write JSON to GCS
                match write_json_to_gcs(
                    self.token.clone(),
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    json,
                )
                .await
                {
                    Ok(filename) => {
                        // Save CDN link to model if successful
                        self.model.cdn_json_uri = Some(format!("{}/{}", self.cdn_prefix, filename));
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
                // Increment retry count for JSON
                self.model.json_parser_retry_count += 1;
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "{}",
                    e.to_string()
                )
            },
        };
    }

    async fn handle_image_optimizer(
        &mut self,
        img_uri: String,
        animation_uri_option: Option<String>,
    ) {
        // Optimize image
        match self.optimize_image(img_uri).await {
            Ok((new_img, format, raw_image_uri)) => {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Successfully optimized image"
                );

                self.model.raw_image_uri = Some(raw_image_uri);

                // Write image to GCS
                match write_image_to_gcs(
                    self.token.clone(),
                    format,
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    new_img,
                )
                .await
                {
                    // Save CDN link to model if successful
                    Ok(filename) => {
                        self.model.cdn_image_uri =
                            Some(format!("{}/{}", self.cdn_prefix, filename));
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
                // Increment retry count for image
                self.model.image_optimizer_retry_count += 1;
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "{}",
                    e.to_string()
                )
            },
        };

        // Optimize animation if exists
        if let Some(animation_uri) = animation_uri_option {
            match self.optimize_image(animation_uri).await {
                Ok((new_img, format, raw_animation_uri)) => {
                    info!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "Successfully optimized image"
                    );

                    self.model.raw_animation_uri = Some(raw_animation_uri);

                    // Write animation to GCS
                    match write_image_to_gcs(
                        self.token.clone(),
                        format,
                        self.bucket.clone(),
                        self.entry.token_data_id.clone(),
                        new_img,
                    )
                    .await
                    {
                        // Save CDN link to model if successful
                        Ok(filename) => {
                            self.model.cdn_animation_uri =
                                Some(format!("{}/{}", self.cdn_prefix, filename));
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
                    // Do not increment retry count for animation
                    error!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "{}",
                        e.to_string()
                    )
                },
            };
        }
    }

    // Calls and handles error for URI parser
    fn handle_uri_parser_jsons(&mut self) -> String {
        match Self::parse_uri(self.entry.token_uri.clone()) {
            Ok(u) => u,
            Err(_) => self.entry.token_uri.clone(),
        }
    }

    // Calls and handles error for URI parser for image and animation URIs
    fn handle_uri_parser_images(&mut self) -> (String, Option<String>) {
        // Use token_uri if  not provided or the JSON parsing failed
        let raw_img_uri = self
            .model
            .raw_image_uri
            .clone()
            .unwrap_or(self.model.token_uri.clone());

        // Parse URI to handle IPFS URIs
        let img_uri = match Self::parse_uri(raw_img_uri.clone()) {
            Ok(u) => u,
            Err(_) => raw_img_uri,
        };

        // Skip if animation URI is not provided
        let animation_uri = self
            .model
            .raw_animation_uri
            .clone()
            .and_then(|raw_animation_uri| Self::parse_uri(raw_animation_uri).ok());

        (img_uri, animation_uri)
    }

    // Calls and handles error for upserting to Postgres
    async fn save_to_postgres(&mut self) {
        match upsert_uris(&mut self.conn, self.model.clone()) {
            Ok(_) => info!(
                last_transaction_version = self.entry.last_transaction_version,
                "Successfully upserted URIs"
            ),
            Err(e) => error!(
                last_transaction_version = self.entry.last_transaction_version,
                "{}",
                e.to_string()
            ),
        };
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
    async fn optimize_image(
        &mut self,
        _img_uri: String,
    ) -> anyhow::Result<(Vec<u8>, ImageFormat, String)> {
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
