// Copyright © Aptos Foundation

use crate::{get_uri_metadata, utils::constants::MAX_RETRY_TIME_SECONDS};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use image::{
    imageops::{resize, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, ImageOutputFormat,
};
use reqwest::Client;
use std::{io::Cursor, time::Duration};
use tracing::error;

pub struct ImageOptimizer;

impl ImageOptimizer {
    /// Resizes and optimizes image from input URI.
    /// Returns new image as a byte array and its format.
    pub async fn optimize(
        uri: String,
        max_file_size_bytes: u32,
        image_quality: u8,
    ) -> anyhow::Result<(Vec<u8>, ImageFormat)> {
        let (_, size) = get_uri_metadata(uri.clone()).await?;
        if size > max_file_size_bytes {
            let error_msg = format!(
                "Image optimizer received file too large: {} bytes, skipping",
                size
            );
            error!(uri = uri, "[NFT Metadata Crawler] {}", error_msg);
            return Err(anyhow::anyhow!(error_msg));
        }

        let op = || {
            async {
                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_RETRY_TIME_SECONDS / 3))
                    .build()
                    .context("Failed to build reqwest client")?;

                let response = client
                    .get(&uri)
                    .send()
                    .await
                    .context("Failed to get image")?;

                let img_bytes = response
                    .bytes()
                    .await
                    .context("Failed to load image bytes")?;

                let format =
                    image::guess_format(&img_bytes).context("Failed to guess image format")?;

                match format {
                    ImageFormat::Gif | ImageFormat::Avif => Ok((img_bytes.to_vec(), format)),
                    _ => {
                        let img = image::load_from_memory(&img_bytes)
                            .context(format!("Failed to load image from memory: {} bytes", size))?;
                        let resized_image = resize(&img.to_rgb8(), 400, 400, FilterType::Gaussian);
                        Ok((Self::to_json_bytes(resized_image, image_quality)?, format))
                    },
                }
            }
            .boxed()
        };

        let backoff = ExponentialBackoff {
            max_elapsed_time: Some(Duration::from_secs(MAX_RETRY_TIME_SECONDS)),
            ..Default::default()
        };

        match retry(backoff, op).await {
            Ok(result) => Ok(result),
            Err(e) => {
                error!(
                    uri = uri,
                    "[NFT Metadata Crawler] Exponential backoff timed out, skipping image"
                );
                Err(e)
            },
        }
    }

    /// Converts image to JPEG bytes vector
    fn to_json_bytes(
        image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>,
        image_quality: u8,
    ) -> anyhow::Result<Vec<u8>> {
        let dynamic_image = DynamicImage::ImageRgb8(image_buffer);
        let mut byte_store = Cursor::new(Vec::new());
        match dynamic_image.write_to(&mut byte_store, ImageOutputFormat::Jpeg(image_quality)) {
            Ok(_) => Ok(byte_store.into_inner()),
            Err(e) => {
                error!(error = ?e, "[NFT Metadata Crawler] Error converting image to bytes:: {} bytes", dynamic_image.as_bytes().len());
                Err(anyhow::anyhow!(e))
            },
        }
    }
}
