// Copyright © Aptos Foundation

use google_cloud_auth::project::{create_token_source, Config};
use image::ImageFormat;
use reqwest::{
    header::{self, HeaderMap},
    Client,
};
use serde_json::Value;

/// Writes JSON Value to GCS
pub async fn write_json_to_gcs(bucket: String, id: String, json: Value) -> anyhow::Result<String> {
    let (client, filename, url) = init_client_and_format_url(id, bucket);
    let json_string = json.to_string();

    let res = client
        .post(url)
        .bearer_auth(get_gcp_auth_token().await?)
        .header("Content-Type", "application/json")
        .body(json_string)
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(filename),
        _ => {
            let text = res.text().await?;
            Err(anyhow::anyhow!("Error saving JSON to GCS {}", text))
        },
    }
}

/// Infers file type and writes image to GCS
pub async fn write_image_to_gcs(
    img_format: ImageFormat,
    bucket: String,
    id: String,
    buffer: Vec<u8>,
) -> anyhow::Result<String> {
    let (client, filename, url) = init_client_and_format_url(id, bucket);
    let mut headers = HeaderMap::new();

    let extension = match img_format {
        ImageFormat::Gif | ImageFormat::Avif => img_format
            .extensions_str()
            .last()
            .unwrap_or(&"gif")
            .to_string(),
        _ => "jpeg".to_string(),
    };

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
        .bearer_auth(get_gcp_auth_token().await?)
        .headers(headers)
        .body(buffer)
        .send()
        .await?;

    match res.status().as_u16() {
        200..=299 => Ok(filename),
        _ => {
            let text = res.text().await?;
            Err(anyhow::anyhow!("Error saving image to GCS {}", text))
        },
    }
}

async fn get_gcp_auth_token() -> anyhow::Result<String> {
    let config = Config {
        audience: None,
        scopes: Some(&["https://www.googleapis.com/auth/cloud-platform"]),
        sub: None,
    };
    let ts = create_token_source(config).await?;
    Ok(ts.token().await?.access_token)
}

/// Creates the request client and formats the filename and URL
fn init_client_and_format_url(id: String, bucket: String) -> (Client, String, String) {
    let client = Client::new();
    let filename = format!("json_{}.json", id);
    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
        bucket, filename
    );
    (client, filename, url)
}
