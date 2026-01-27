// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};

/// Required account data and auth keys for Cloudflare
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetUploaderWorkerConfig {
    /// Cloudflare API key
    pub cloudflare_auth_key: String,
    /// Cloudflare Account ID provided at the images home page used to authenticate requests
    pub cloudflare_account_id: String,
}
