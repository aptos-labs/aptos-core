// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetUploaderThrottlerConfig {
    /// URI for the Asset Uploader Worker
    pub asset_uploader_worker_uri: String,
    /// Interval in seconds to poll Postgres to update upload queue
    #[serde(default = "AssetUploaderThrottlerConfig::default_poll_interval_seconds")]
    pub poll_interval_seconds: u64,
    /// Maximum number of rows to poll from Postgres
    #[serde(default = "AssetUploaderThrottlerConfig::default_poll_rows_limit")]
    pub poll_rows_limit: u64,
    /// Cloudflare Account Hash provided at the images home page used for generating the CDN image URLs
    pub cloudflare_account_hash: String,
    /// Cloudflare Image Delivery URL prefix provided at the images home page used for generating the CDN image URLs
    pub cloudflare_image_delivery_prefix: String,
    /// In addition to on the fly transformations, Cloudflare images can be returned in preset variants. This is the default variant used with the saved CDN image URLs.
    pub cloudflare_default_variant: String,
}

impl AssetUploaderThrottlerConfig {
    pub const fn default_poll_interval_seconds() -> u64 {
        10
    }

    pub const fn default_poll_rows_limit() -> u64 {
        600
    }
}
