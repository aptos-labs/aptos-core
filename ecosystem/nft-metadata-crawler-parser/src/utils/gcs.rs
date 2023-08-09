// Copyright © Aptos Foundation

use anyhow::Context;
use google_cloud_storage::{
    client::{Client, ClientConfig},
    http::objects::upload::{Media, UploadObjectRequest, UploadType},
};
use image::ImageFormat;
use serde_json::Value;

/// Writes JSON Value to GCS
pub async fn write_json_to_gcs(bucket: String, id: String, json: Value) -> anyhow::Result<String> {
    let client = init_client().await?;

    let filename = format!("{}/json.json", id);
    let json_string = json.to_string();
    let json_bytes = json_string.into_bytes();

    let upload_type = UploadType::Simple(Media {
        name: filename.clone().into(),
        content_type: "application/json".into(),
        content_length: Some(json_bytes.len() as u64),
    });

    client
        .upload_object(
            &UploadObjectRequest {
                bucket,
                ..Default::default()
            },
            json_bytes,
            &upload_type,
        )
        .await
        .context("Error uploading JSON to GCS")?;

    Ok(filename)
}

/// Infers file type and writes image to GCS
pub async fn write_image_to_gcs(
    img_format: ImageFormat,
    bucket: String,
    id: String,
    buffer: Vec<u8>,
) -> anyhow::Result<String> {
    let client = init_client().await?;

    let extension = match img_format {
        ImageFormat::Gif | ImageFormat::Avif => img_format
            .extensions_str()
            .last()
            .unwrap_or(&"gif")
            .to_string(),
        _ => "jpeg".to_string(),
    };

    let filename = format!("{}/image.{}", id, extension);

    let upload_type = UploadType::Simple(Media {
        name: filename.clone().into(),
        content_type: format!("image/{}", extension).into(),
        content_length: Some(buffer.len() as u64),
    });

    client
        .upload_object(
            &UploadObjectRequest {
                bucket,
                ..Default::default()
            },
            buffer,
            &upload_type,
        )
        .await
        .context("Error uploading image to GCS")?;

    Ok(filename)
}

/// Creates a GCS client using auth from env variable
async fn init_client() -> anyhow::Result<Client> {
    let config = ClientConfig::default().with_auth().await?;
    Ok(Client::new(config))
}
