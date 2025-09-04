// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::parsed_asset_uris_query::ParsedAssetUrisQuery,
    schema::nft_metadata_crawler::parsed_asset_uris,
};
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(asset_uri))]
#[diesel(table_name = parsed_asset_uris)]
pub struct ParsedAssetUris {
    asset_uri: String,
    raw_image_uri: Option<String>,
    raw_animation_uri: Option<String>,
    cdn_json_uri: Option<String>,
    cdn_image_uri: Option<String>,
    cdn_animation_uri: Option<String>,
    json_parser_retry_count: i32,
    image_optimizer_retry_count: i32,
    animation_optimizer_retry_count: i32,
    do_not_parse: bool,
    last_transaction_version: i64,
}

impl ParsedAssetUris {
    pub fn new(asset_uri: &str) -> Self {
        Self {
            asset_uri: asset_uri.to_string(),
            raw_image_uri: None,
            raw_animation_uri: None,
            cdn_json_uri: None,
            cdn_image_uri: None,
            cdn_animation_uri: None,
            json_parser_retry_count: 0,
            image_optimizer_retry_count: 0,
            animation_optimizer_retry_count: 0,
            do_not_parse: false,
            last_transaction_version: 0,
        }
    }

    pub fn get_asset_uri(&self) -> String {
        self.asset_uri.clone()
    }

    pub fn set_asset_uri(&mut self, asset_uri: String) {
        self.asset_uri = asset_uri;
    }

    pub fn get_raw_image_uri(&self) -> Option<String> {
        let uri = self.raw_image_uri.clone();
        if uri.is_none() {
            warn!(
                asset_uri = self.asset_uri,
                "[NFT Metadata Crawler] raw_image_uri is None"
            );
        }
        uri
    }

    pub fn set_raw_image_uri(&mut self, raw_image_uri: Option<String>) {
        self.raw_image_uri = raw_image_uri;
    }

    pub fn get_raw_animation_uri(&self) -> Option<String> {
        let uri = self.raw_animation_uri.clone();
        if uri.is_none() {
            warn!(
                asset_uri = self.asset_uri,
                "[NFT Metadata Crawler] raw_animation_uri is None"
            );
        }
        uri
    }

    pub fn set_raw_animation_uri(&mut self, raw_animation_uri: Option<String>) {
        self.raw_animation_uri = raw_animation_uri;
    }

    pub fn get_cdn_json_uri(&self) -> Option<String> {
        let uri = self.cdn_json_uri.clone();
        if uri.is_none() {
            warn!(
                asset_uri = self.asset_uri,
                "[NFT Metadata Crawler] cdn_json_uri is None"
            );
        }
        uri
    }

    pub fn set_cdn_json_uri(&mut self, cdn_json_uri: Option<String>) {
        self.cdn_json_uri = cdn_json_uri;
    }

    pub fn get_cdn_image_uri(&self) -> Option<String> {
        let uri = self.cdn_image_uri.clone();
        if uri.is_none() {
            warn!(
                asset_uri = self.asset_uri,
                "[NFT Metadata Crawler] cdn_image_uri is None"
            );
        }
        uri
    }

    pub fn set_cdn_image_uri(&mut self, cdn_image_uri: Option<String>) {
        self.cdn_image_uri = cdn_image_uri;
    }

    pub fn get_cdn_animation_uri(&self) -> Option<String> {
        let uri = self.cdn_animation_uri.clone();
        if uri.is_none() {
            warn!(
                asset_uri = self.asset_uri,
                "[NFT Metadata Crawler] cdn_animation_uri is None"
            );
        }
        uri
    }

    pub fn set_cdn_animation_uri(&mut self, cdn_animation_uri: Option<String>) {
        self.cdn_animation_uri = cdn_animation_uri;
    }

    pub fn get_json_parser_retry_count(&self) -> i32 {
        self.json_parser_retry_count
    }

    pub fn increment_json_parser_retry_count(&mut self) {
        self.json_parser_retry_count += 1;
    }

    pub fn reset_json_parser_retry_count(&mut self) {
        self.json_parser_retry_count = 0;
    }

    pub fn get_image_optimizer_retry_count(&self) -> i32 {
        self.image_optimizer_retry_count
    }

    pub fn increment_image_optimizer_retry_count(&mut self) {
        self.image_optimizer_retry_count += 1;
    }

    pub fn get_animation_optimizer_retry_count(&self) -> i32 {
        self.animation_optimizer_retry_count
    }

    pub fn increment_animation_optimizer_retry_count(&mut self) {
        self.animation_optimizer_retry_count += 1;
    }

    pub fn get_do_not_parse(&self) -> bool {
        self.do_not_parse
    }

    pub fn set_do_not_parse(&mut self, do_not_parse: bool) {
        self.do_not_parse = do_not_parse;
    }

    pub fn get_last_transaction_version(&self) -> i64 {
        self.last_transaction_version
    }

    pub fn set_last_transaction_version(&mut self, last_transaction_version: i64) {
        self.last_transaction_version = last_transaction_version;
    }
}

impl From<ParsedAssetUrisQuery> for ParsedAssetUris {
    fn from(query: ParsedAssetUrisQuery) -> Self {
        Self {
            asset_uri: query.asset_uri,
            raw_image_uri: query.raw_image_uri,
            raw_animation_uri: query.raw_animation_uri,
            cdn_json_uri: query.cdn_json_uri,
            cdn_image_uri: query.cdn_image_uri,
            cdn_animation_uri: query.cdn_animation_uri,
            json_parser_retry_count: query.json_parser_retry_count,
            image_optimizer_retry_count: query.image_optimizer_retry_count,
            animation_optimizer_retry_count: query.animation_optimizer_retry_count,
            do_not_parse: query.do_not_parse,
            last_transaction_version: query.last_transaction_version,
        }
    }
}
