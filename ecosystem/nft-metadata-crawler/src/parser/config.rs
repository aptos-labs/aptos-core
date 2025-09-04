// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::constants::{
    DEFAULT_IMAGE_QUALITY, DEFAULT_MAX_FILE_SIZE_BYTES, DEFAULT_MAX_IMAGE_DIMENSIONS,
    DEFAULT_MAX_NUM_PARSE_RETRIES,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    pub google_application_credentials: Option<String>,
    pub bucket: String,
    pub cdn_prefix: String,
    pub ipfs_prefix: String,
    pub ipfs_auth_key: Option<String>,
    #[serde(default = "ParserConfig::default_max_file_size_bytes")]
    pub max_file_size_bytes: u32,
    #[serde(default = "ParserConfig::default_image_quality")]
    pub image_quality: u8, // Quality up to 100
    #[serde(default = "ParserConfig::default_max_image_dimensions")]
    pub max_image_dimensions: u32,
    #[serde(default = "ParserConfig::default_max_num_parse_retries")]
    pub max_num_parse_retries: i32,
    #[serde(default)]
    pub ack_parsed_uris: bool,
    #[serde(default)]
    pub uri_blacklist: Vec<String>,
}

impl ParserConfig {
    pub const fn default_max_file_size_bytes() -> u32 {
        DEFAULT_MAX_FILE_SIZE_BYTES
    }

    pub const fn default_image_quality() -> u8 {
        DEFAULT_IMAGE_QUALITY
    }

    pub const fn default_max_image_dimensions() -> u32 {
        DEFAULT_MAX_IMAGE_DIMENSIONS
    }

    pub const fn default_max_num_parse_retries() -> i32 {
        DEFAULT_MAX_NUM_PARSE_RETRIES
    }
}
