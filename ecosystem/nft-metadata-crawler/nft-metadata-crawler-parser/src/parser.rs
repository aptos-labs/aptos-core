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
use nft_metadata_crawler_utils::NFTMetadataCrawlerEntry;
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
        // set token_uri

        let (_raw_image_uri, _raw_animation_uri, _json) = self.handle_json_parser(json_uri).await;
        // set raw_image_uri, raw_animation_uri

        // save json to gcs
        // set cdn_json_uri

        self.save_to_postgres().await;

        let (img_uri, animation_uri) = self.handle_uri_parser_images();

        let image = self.handle_image_optimizer(img_uri).await;
        let animation = self.handle_animation_downloader(animation_uri).await;

        // save image and animation to gcs
        // set cdn_image_uri and cdn_animation_uri

        self.save_to_postgres().await;

        Ok(())
    }

    // Calls and handles error for JSON parser
    async fn handle_json_parser(
        &mut self,
        json_uri: String,
    ) -> (Option<String>, Option<String>, Option<Value>) {
        if let Ok((mime, _size)) = self.get_url_metadata(json_uri.clone()).await {
            if let Some(_) = ImageFormat::from_mime_type(mime) {
                return (None, None, None);
            }

            match self.parse_json(json_uri).await {
                Ok(json) => {
                    info!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "Successfully parsed JSON"
                    );

                    let raw_image_uri =
                        json.get("image").and_then(Value::as_str).map(str::to_owned);
                    let raw_animation_uri = json
                        .get("animation")
                        .and_then(Value::as_str)
                        .map(str::to_owned);

                    return (raw_image_uri, raw_animation_uri, Some(json));
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
        (None, None, None)
    }

    async fn handle_image_optimizer(&mut self, img_uri: String) -> Option<(Vec<u8>, ImageFormat)> {
        // Optimize image
        match self.optimize_image(img_uri).await {
            Ok((new_img, format, raw_image_uri)) => {
                info!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "Successfully optimized image"
                );

                return Some((new_img, format));
            },
            Err(e) => {
                // Increment retry count for image
                self.model.image_optimizer_retry_count += 1;
                error!(
                    last_transaction_version = self.entry.last_transaction_version,
                    "{}",
                    e.to_string()
                );
                return None;
            },
        };
    }

    async fn handle_animation_downloader(
        &mut self,
        animation_uri_option: Option<String>,
    ) -> Option<(Vec<u8>, ImageFormat)> {
        // Optimize animation if exists
        if let Some(animation_uri) = animation_uri_option {
            match self.optimize_image(animation_uri).await {
                Ok((new_img, format, raw_animation_uri)) => {
                    info!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "Successfully optimized image"
                    );
                    return Some((new_img, format));
                },
                Err(e) => {
                    // Do not increment retry count for animation
                    error!(
                        last_transaction_version = self.entry.last_transaction_version,
                        "{}",
                        e.to_string()
                    );
                },
            };
        }
        return None;
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

    // HEAD request to get MIME type and size of content
    async fn get_url_metadata(&mut self, _url: String) -> anyhow::Result<(String, u32)> {
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
