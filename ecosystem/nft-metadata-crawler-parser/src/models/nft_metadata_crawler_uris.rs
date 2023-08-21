// Copyright © Aptos Foundation

use crate::schema::nft_metadata_crawler::parsed_token_uris;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_uri))]
#[diesel(table_name = parsed_token_uris)]
pub struct NFTMetadataCrawlerURIs {
    token_uri: String,
    raw_image_uri: Option<String>,
    raw_animation_uri: Option<String>,
    cdn_json_uri: Option<String>,
    cdn_image_uri: Option<String>,
    cdn_animation_uri: Option<String>,
    json_parser_retry_count: i32,
    image_optimizer_retry_count: i32,
    animation_optimizer_retry_count: i32,
}

impl NFTMetadataCrawlerURIs {
    pub fn new(token_uri: String) -> Self {
        Self {
            token_uri,
            raw_image_uri: None,
            raw_animation_uri: None,
            cdn_json_uri: None,
            cdn_image_uri: None,
            cdn_animation_uri: None,
            json_parser_retry_count: 0,
            image_optimizer_retry_count: 0,
            animation_optimizer_retry_count: 0,
        }
    }

    pub fn get_token_uri(&self) -> String {
        self.token_uri.clone()
    }

    pub fn set_token_uri(&mut self, token_uri: String) {
        self.token_uri = token_uri;
    }

    pub fn get_raw_image_uri(&self) -> Option<String> {
        let uri = self.raw_image_uri.clone();
        if uri.is_none() {
            warn!(
                token_uri = self.token_uri,
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
                token_uri = self.token_uri,
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
                token_uri = self.token_uri,
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
                token_uri = self.token_uri,
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
                token_uri = self.token_uri,
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
}
