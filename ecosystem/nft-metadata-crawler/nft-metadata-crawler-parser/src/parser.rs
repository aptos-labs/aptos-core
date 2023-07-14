// Copyright © Aptos Foundation

use crate::{db::upsert_uris, models::NFTMetadataCrawlerURIs, schema::nft_metadata_crawler_uris};
use chrono::Utc;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection, QueryDsl, RunQueryDsl,
};
use google_cloud_auth::token_source::TokenSource;
use image::{ImageBuffer, ImageFormat};
use nft_metadata_crawler_utils::{
    gcs::{write_image_to_gcs, write_json_to_gcs},
    NFTMetadataCrawlerEntry,
};
use serde_json::Value;
use std::error::Error;

pub struct Parser<'a> {
    pub entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    format: ImageFormat,
    bucket: String,
    ts: &'a dyn TokenSource,
    force: bool,
}

impl<'a> Parser<'a> {
    pub fn new(e: NFTMetadataCrawlerEntry, b: String, f: bool, t: &'a dyn TokenSource) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs {
                token_uri: e.token_uri.clone(),
                raw_image_uri: None,
                raw_animation_uri: None,
                cdn_json_uri: None,
                cdn_image_uri: None,
                cdn_animation_uri: None,
                image_resizer_retry_count: 0,
                json_parser_retry_count: 0,
                last_updated: Utc::now().naive_utc(),
            },
            entry: e,
            format: ImageFormat::Jpeg,
            bucket: b,
            ts: t,
            force: f,
        }
    }

    pub async fn parse(
        &mut self,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        if nft_metadata_crawler_uris::table
            .find(&self.entry.token_uri)
            .first::<NFTMetadataCrawlerURIs>(conn)
            .is_ok()
        {
            if self.force {
                self.log("Found URIs entry but forcing parse");
            } else {
                self.log("Skipping URI parse");
                return Ok(());
            }
        }

        // URI Parser
        let json_uri = match Self::parse_uri(self.entry.token_uri.clone()) {
            Ok(u) => u,
            Err(_) => self.entry.token_uri.clone(),
        };

        // JSON Parser
        match self.parse_json(json_uri).await {
            Ok(json) => {
                self.log("Successfully parsed JSON");

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
                        self.model.cdn_json_uri =
                            Some(format!("http://34.160.26.161/{}", filename));
                        self.log("Successfully saved JSON")
                    },
                    Err(e) => self.log(&e.to_string()),
                }
            },
            Err(e) => {
                self.model.json_parser_retry_count += 1;
                self.log(&e.to_string())
            },
        }

        // Save to Postgres
        match upsert_uris(conn, self.model.clone()) {
            Ok(_) => self.log("Successfully upserted JSON URIs"),
            Err(e) => self.log(&e.to_string()),
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
                self.log("Successfully optimized image");

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
                        self.log("Successfully saved image");
                    },
                    Err(e) => self.log(&e.to_string()),
                }
            },
            Err(e) => {
                self.model.image_resizer_retry_count += 1;
                self.log(&e.to_string())
            },
        }

        // Save to Postgres
        match upsert_uris(conn, self.model.clone()) {
            Ok(_) => self.log("Successfully upserted image URIs"),
            Err(e) => self.log(&e.to_string()),
        }

        Ok(())
    }

    fn parse_uri(_uri: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    async fn _get_size(&mut self, _url: String) -> Result<u32, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    async fn parse_json(&mut self, _uri: String) -> Result<Value, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    async fn optimize_image(
        &mut self,
        _img_uri: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    // Function that adds correct bytes to image
    fn _to_bytes(
        &self,
        _image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        todo!();
    }

    // Function to help with logging
    fn log(&self, _message: &str) {
        todo!();
    }
}
