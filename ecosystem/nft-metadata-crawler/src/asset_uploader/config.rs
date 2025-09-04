// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Required account data and auth keys for Cloudflare
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetUploaderConfig {
    /// Cloudflare API key
    pub cloudflare_auth_key: String,
    /// Cloudflare Account ID provided at the images home page used to authenticate requests
    pub cloudflare_account_id: String,
    /// Cloudflare Account Hash provided at the images home page used for generating the CDN image URLs
    pub cloudflare_account_hash: String,
    /// Cloudflare Image Delivery URL prefix provided at the images home page used for generating the CDN image URLs
    pub cloudflare_image_delivery_prefix: String,
    /// In addition to on the fly transformations, Cloudflare images can be returned in preset variants. This is the default variant used with the saved CDN image URLs.
    pub cloudflare_default_variant: String,
}
