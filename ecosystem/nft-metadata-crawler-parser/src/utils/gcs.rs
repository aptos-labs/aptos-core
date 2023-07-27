// Copyright © Aptos Foundation

use image::ImageFormat;
use serde_json::Value;

pub async fn write_json_to_gcs(
    _bucket: String,
    _id: String,
    _json: Value,
) -> anyhow::Result<String> {
    todo!();
}

pub async fn write_image_to_gcs(
    _img_format: ImageFormat,
    _bucket: String,
    _id: String,
    _buffer: Vec<u8>,
) -> anyhow::Result<String> {
    todo!();
}
