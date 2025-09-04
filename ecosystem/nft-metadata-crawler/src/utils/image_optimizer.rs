// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_uri_metadata,
    utils::{
        constants::{MAX_IMAGE_REQUEST_RETRY_SECONDS, MAX_RETRY_TIME_SECONDS},
        counters::{
            FAILED_TO_OPTIMIZE_IMAGE_COUNT, OPTIMIZE_IMAGE_INVOCATION_COUNT,
            SUCCESSFULLY_OPTIMIZED_IMAGE_COUNT,
        },
    },
};
use anyhow::Context;
use backoff::{future::retry, ExponentialBackoff};
use futures::FutureExt;
use image::{
    imageops::{resize, FilterType},
    DynamicImage, GenericImageView, ImageBuffer, ImageFormat, ImageOutputFormat,
};
use reqwest::Client;
use std::{
    cmp::{max, min},
    io::Cursor,
    time::Duration,
};
use tracing::{info, warn};

pub struct ImageOptimizer;

impl ImageOptimizer {
    /// Resizes and optimizes image from input URI.
    /// Returns new image as a byte array and its format.
    pub async fn optimize(
        uri: &str,
        max_file_size_bytes: u32,
        image_quality: u8,
        max_image_dimensions: u32,
    ) -> anyhow::Result<(Vec<u8>, ImageFormat)> {
        OPTIMIZE_IMAGE_INVOCATION_COUNT.inc();
        let (_, size) = get_uri_metadata(uri).await?;
        if size > max_file_size_bytes {
            FAILED_TO_OPTIMIZE_IMAGE_COUNT
                .with_label_values(&["Image file too large"])
                .inc();
            return Err(anyhow::anyhow!(format!(
                "Image optimizer received file too large: {} bytes, skipping",
                size
            )));
        }

        let op = || {
            async {
                info!(image_uri = uri, "Sending request for image");

                let client = Client::builder()
                    .timeout(Duration::from_secs(MAX_IMAGE_REQUEST_RETRY_SECONDS))
                    .build()
                    .context("Failed to build reqwest client")?;

                let response = client
                    .get(uri.trim())
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
                        let (nwidth, nheight) = Self::calculate_dimensions_with_ration(
                            min(max(img.width(), img.height()), max_image_dimensions),
                            img.width(),
                            img.height(),
                        );
                        let resized_image =
                            resize(&img.to_rgba8(), nwidth, nheight, FilterType::Gaussian);
                        Ok(Self::to_image_bytes(resized_image, image_quality)?)
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
            Ok(result) => {
                SUCCESSFULLY_OPTIMIZED_IMAGE_COUNT.inc();
                Ok(result)
            },
            Err(e) => {
                FAILED_TO_OPTIMIZE_IMAGE_COUNT
                    .with_label_values(&["other"])
                    .inc();
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

    /// Checks if an image has any transparent pixels
    fn has_transparent_pixels(img: &DynamicImage) -> bool {
        let (width, height) = img.dimensions();
        for x in 0..width {
            for y in 0..height {
                if img.get_pixel(x, y)[3] < 255 {
                    return true;
                }
            }
        }
        false
    }

    /// Converts image to image bytes vector
    fn to_image_bytes(
        image_buffer: ImageBuffer<image::Rgba<u8>, Vec<u8>>,
        image_quality: u8,
    ) -> anyhow::Result<(Vec<u8>, ImageFormat)> {
        let dynamic_image = DynamicImage::ImageRgba8(image_buffer);
        let mut byte_store = Cursor::new(Vec::new());
        let mut encode_format = ImageOutputFormat::Jpeg(image_quality);
        let mut output_format = ImageFormat::Jpeg;
        if Self::has_transparent_pixels(&dynamic_image) {
            encode_format = ImageOutputFormat::Png;
            output_format = ImageFormat::Png;
        }

        match dynamic_image.write_to(&mut byte_store, encode_format) {
            Ok(_) => Ok((byte_store.into_inner(), output_format)),
            Err(e) => {
                warn!(error = ?e, "[NFT Metadata Crawler] Error converting image to bytes: {} bytes", dynamic_image.as_bytes().len());
                Err(anyhow::anyhow!(e))
            },
        }
    }
}
