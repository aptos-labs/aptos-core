// Copyright © Aptos Foundation

use anyhow::anyhow;
use image::ImageFormat;
use reqwest::{
    header::{self, HeaderMap},
    Client,
};
use serde_json::Value;

pub async fn write_json_to_gcs(
    token: String,
    bucket: String,
    id: String,
    json: Value,
) -> anyhow::Result<String> {
    let client = Client::new();
    let filename = format!("json_{}.json", id);
    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
        bucket, filename
    );
    let json_string = json.to_string();

    let res = client
        .post(url)
        .bearer_auth(token)
        .header("Content-Type", "application/json")
        .body(json_string)
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(filename),
        _ => {
            let text = res.text().await?;
            Err(anyhow!("Error saving JSON to GCS {}", text))
        },
    }
}

pub async fn write_image_to_gcs(
    token: String,
    img_format: ImageFormat,
    bucket: String,
    id: String,
    buffer: Vec<u8>,
) -> anyhow::Result<String> {
    let client = Client::new();
    let mut headers = HeaderMap::new();

    let extension = match img_format {
        ImageFormat::Gif | ImageFormat::Avif => img_format
            .extensions_str()
            .last()
            .unwrap_or(&"gif")
            .to_string(),
        _ => "jpeg".to_string(),
    };

    let filename = format!("image_{}.{}", id, extension);
    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
        bucket, filename
    );

    headers.insert(
        header::CONTENT_TYPE,
        format!("image/{}", extension).parse().unwrap(),
    );

    headers.insert(
        header::CONTENT_LENGTH,
        buffer.len().to_string().parse().unwrap(),
    );

    let res = client
        .post(&url)
        .bearer_auth(token)
        .headers(headers)
        .body(buffer)
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(filename),
        _ => {
            let text = res.text().await?;
            Err(anyhow!("Error saving image to GCS {}", text))
        },
    }
}
