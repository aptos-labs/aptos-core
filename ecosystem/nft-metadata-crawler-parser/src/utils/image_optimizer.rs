// Copyright © Aptos Foundation

use crate::{
    get_uri_metadata,
    utils::constants::{MAX_IMAGE_REQUEST_RETRY_SECONDS, MAX_RETRY_TIME_SECONDS},
};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use image::{
    imageops::{resize, FilterType},
    DynamicImage, ImageBuffer, ImageFormat, ImageOutputFormat,
};
use reqwest::Client;
use std::{io::Cursor, time::Duration};
use tracing::warn;

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
            return Err(anyhow::anyhow!(format!(
                "Image optimizer received file too large: {} bytes, skipping",
                size
            )));
        }

        let op = || {
            async {
                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_IMAGE_REQUEST_RETRY_SECONDS))
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
                        let (nwidth, nheight) =
                            Self::calculate_dimensions_with_ration(512, img.width(), img.height());
                        let resized_image =
                            resize(&img.to_rgb8(), nwidth, nheight, FilterType::Gaussian);
                        Ok((Self::to_jpeg_bytes(resized_image, image_quality)?, format))
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
                warn!(
                    uri = uri,
                    error = ?e,
                    "[NFT Metadata Crawler] Exponential backoff timed out, skipping image"
                );
                Err(e)
            },
        }
    }

    /// Calculate new dimensions given a goal size while maintaining original aspect ratio
    fn calculate_dimensions_with_ration(goal: u32, width: u32, height: u32) -> (u32, u32) {
        if width == 0 || height == 0 {
            return (0, 0);
        }

        if width > height {
            let new_width = goal;
            let new_height = (goal as f64 * (height as f64 / width as f64)).round() as u32;
            (new_width, new_height)
        } else {
            let new_height = goal;
            let new_width = (goal as f64 * (width as f64 / height as f64)).round() as u32;
            (new_width, new_height)
        }
    }

    /// Converts image to JPEG bytes vector
    fn to_jpeg_bytes(
        image_buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>>,
        image_quality: u8,
    ) -> anyhow::Result<Vec<u8>> {
        let dynamic_image = DynamicImage::ImageRgb8(image_buffer);
        let mut byte_store = Cursor::new(Vec::new());
        match dynamic_image.write_to(&mut byte_store, ImageOutputFormat::Jpeg(image_quality)) {
            Ok(_) => Ok(byte_store.into_inner()),
            Err(e) => {
                warn!(error = ?e, "[NFT Metadata Crawler] Error converting image to bytes: {} bytes", dynamic_image.as_bytes().len());
                Err(anyhow::anyhow!(e))
            },
        }
    }
}
