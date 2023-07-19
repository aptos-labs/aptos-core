// Copyright © Aptos Foundation

use crate::get_uri_metadata;
use image::ImageFormat;
use serde_json::Value;
use tracing::{error, info};

pub struct JSONParser;

impl JSONParser {
    /**
     * Parses JSON from input URI.
     * Returns the underlying raw image URI, raw animation URI, and JSON.
     */
    pub async fn parse(uri: String) -> (Option<String>, Option<String>, Option<Value>) {
        match Self::parse_json(uri.clone()).await {
            Ok(out) => out,
            Err(e) => {
                error!("Error parsing JSON: {}", e);
                (None, None, None)
            },
        }
    }

    /**
     * Parses JSON from input URI.
     */
    async fn parse_json(
        uri: String,
    ) -> anyhow::Result<(Option<String>, Option<String>, Option<Value>)> {
        let (mime, size) = get_uri_metadata(uri.clone()).await?;
        if ImageFormat::from_mime_type(mime).is_some() {
            error!("JSON parser received image URI, skipping");
            return Ok((None, None, None));
        } else if size > 5000000 {
            error!("JSON parser received large file, skipping");
            return Ok((None, None, None));
        }

        for _ in 0..3 {
            info!("Sending request for token_uri {}", uri);

            let result = reqwest::get(&uri).await?;
            let parsed_json = result.json::<Value>().await?;

            let raw_image_uri = parsed_json["image"].as_str().map(|s| s.to_string());
            let raw_animation_uri = parsed_json["animation_url"].as_str().map(|s| s.to_string());

            return Ok((raw_image_uri, raw_animation_uri, Some(parsed_json)));
        }
        Err(anyhow::anyhow!("Error sending request x3, skipping JSON"))
    }
}
