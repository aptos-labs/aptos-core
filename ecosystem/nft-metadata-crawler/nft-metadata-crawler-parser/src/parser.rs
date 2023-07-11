// Copyright © Aptos Foundation

use crate::{
    db::upsert_uris,
    models::{NFTMetadataCrawlerEntry, NFTMetadataCrawlerURIs},
    schema::nft_metadata_crawler_uris,
};
use chrono::Utc;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection, QueryDsl, RunQueryDsl,
};
use google_cloud_auth::token_source::TokenSource;
use image::{
    imageops::{resize, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, ImageOutputFormat,
};
use nft_metadata_crawler_utils::gcs::{write_image_to_gcs, write_json_to_gcs};
use regex::Regex;
use reqwest::Client;
use serde_json::Value;
use std::{error::Error, io::Cursor};
use url::Url;

pub struct Parser<'a> {
    pub entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    format: ImageFormat,
    target_size: (u32, u32),
    bucket: String,
    ts: &'a dyn TokenSource,
    force: bool,
}

impl<'a> Parser<'a> {
    pub fn new(
        e: NFTMetadataCrawlerEntry,
        ts: Option<(u32, u32)>,
        b: String,
        f: bool,
        t: &'a dyn TokenSource,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs {
                token_uri: e.token_uri.clone(),
                raw_image_uri: None,
                cdn_json_uri: None,
                cdn_image_uri: None,
                image_resizer_retry_count: 0,
                json_parser_retry_count: 0,
                last_updated: Utc::now().naive_utc(),
            },
            entry: e,
            format: ImageFormat::Jpeg,
            target_size: ts.unwrap_or((400, 400)),
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

                // Write to GCS
                match write_json_to_gcs(
                    self.ts,
                    self.bucket.clone(),
                    self.entry.token_data_id.clone(),
                    json,
                )
                .await
                {
                    Ok(filename) => {
                        self.model.cdn_image_uri =
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

                // Write to GCS
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

    fn parse_uri(uri: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        let modified_uri = if uri.starts_with("ipfs://") {
            uri.replace("ipfs://", "https://ipfs.com/ipfs/")
        } else {
            uri
        };

        let re = Regex::new(r"^(ipfs/)(?P<cid>[a-zA-Z0-9]+)(?P<path>/.*)?$")?;

        let path = Url::parse(&modified_uri)?
            .path_segments()
            .map(|segments| segments.collect::<Vec<_>>().join("/"));

        if let Some(captures) = re.captures(&path.unwrap_or_default()) {
            let cid = captures["cid"].to_string();
            let path = captures.name("path").map(|m| m.as_str().to_string());

            Ok(format!(
                "https://testlaunchmynft.mypinata.cloud/ipfs/{}{}",
                cid,
                path.unwrap_or_default()
            ))
        } else {
            Err("Invalid IPFS URI".into())
        }
    }

    async fn get_size(&mut self, url: String) -> Result<u32, Box<dyn Error + Send + Sync>> {
        let client = Client::new();
        let header_map = client.head(url).send().await?.headers().clone();
        match header_map.get("content-length") {
            Some(length) => Ok(length.to_str()?.parse::<u32>()?),
            None => Err("No content-length header, skipping".into()),
        }
    }

    async fn parse_json(&mut self, uri: String) -> Result<Value, Box<dyn Error + Send + Sync>> {
        if self.get_size(uri.clone()).await? > 5000000 {
            return Err("File too large, skipping".into());
        }

        for _ in 0..3 {
            self.log(&format!("Sending request for token_uri {}", uri));

            let result: Result<Value, Box<dyn Error + Send + Sync>> = async {
                let response = reqwest::get(&uri).await?;
                let parsed_json = response.json::<Value>().await?;
                if let Some(img) = parsed_json["image"].as_str() {
                    self.model.raw_image_uri = Some(img.to_string());
                    self.model.last_updated = Utc::now().naive_local();
                }
                Ok(parsed_json)
            }
            .await;

            if let Ok(parsed_json) = result {
                return Ok(parsed_json);
            }
        }
        Err("Error sending request x3, skipping JSON".into())
    }

    async fn optimize_image(
        &mut self,
        img_uri: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        if self.get_size(img_uri.clone()).await? > 5000000 {
            return Err("File too large, skipping".into());
        }

        for _ in 0..3 {
            self.log(&format!(
                "Sending request for raw_image_uri {}",
                img_uri.clone()
            ));

            let response = reqwest::get(img_uri.clone()).await?;
            if response.status().is_success() {
                let img_bytes = response.bytes().await?;
                self.model.raw_image_uri = Some(img_uri);
                let format = image::guess_format(img_bytes.as_ref())?;
                self.format = format;
                match format {
                    ImageFormat::Gif | ImageFormat::Avif => return Ok(img_bytes.to_vec()),
                    _ => match image::load_from_memory(&img_bytes) {
                        Ok(img) => {
                            return self.to_bytes(resize(
                                &img.to_rgb8(),
                                self.target_size.0,
                                self.target_size.1,
                                FilterType::Gaussian,
                            ))
                        },
                        Err(e) => {
                            return Err(format!("Error converting image to bytes: {}", e).into());
                        },
                    },
                }
            }
        }
        Err("Error sending request x3, skipping image".into())
    }

    fn to_bytes(
        &self,
        image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    ) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let dynamic_image = DynamicImage::ImageRgb8(image_buffer);
        let mut byte_store = Cursor::new(Vec::new());
        match dynamic_image.write_to(&mut byte_store, ImageOutputFormat::Jpeg(50)) {
            Ok(_) => Ok(byte_store.into_inner()),
            Err(_) => Err("Error converting image to bytes".into()),
        }
    }

    fn log(&self, message: &str) {
        println!(
            "Transaction Version {}: {}",
            self.entry.last_transaction_version, message
        );
    }
}
