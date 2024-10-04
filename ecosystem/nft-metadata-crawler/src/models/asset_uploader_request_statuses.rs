// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::nft_metadata_crawler::asset_uploader_request_statuses;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(request_id, asset_uri))]
#[diesel(table_name = asset_uploader_request_statuses)]
pub struct AssetUploaderRequestStatuses {
    request_id: Uuid,
    asset_uri: String,
    application_id: Uuid,
    status_code: Option<i64>,
    error_message: Option<String>,
    cdn_image_uri: Option<String>,
    num_failures: i64,
}

impl AssetUploaderRequestStatuses {
    pub fn new(request_id: Uuid, asset_uri: &str, application_id: Uuid) -> Self {
        Self {
            request_id,
            asset_uri: asset_uri.to_string(),
            application_id,
            status_code: None,
            error_message: None,
            cdn_image_uri: None,
            num_failures: 0,
        }
    }

    pub fn new_completed(
        request_id: Uuid,
        asset_uri: &str,
        application_id: Uuid,
        cdn_image_uri: &str,
    ) -> Self {
        Self {
            request_id,
            asset_uri: asset_uri.to_string(),
            application_id,
            status_code: Some(200),
            error_message: None,
            cdn_image_uri: Some(cdn_image_uri.to_string()),
            num_failures: 0,
        }
    }

    pub fn get_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn get_asset_uri(&self) -> String {
        self.asset_uri.clone()
    }

    pub fn get_application_id(&self) -> Uuid {
        self.application_id
    }

    pub fn get_status_code(&self) -> Option<i64> {
        self.status_code
    }

    pub fn set_status_code(&mut self, status_code: Option<i64>) {
        self.status_code = status_code;
    }

    pub fn get_error_message(&self) -> Option<String> {
        self.error_message.clone()
    }

    pub fn set_error_message(&mut self, error_message: Option<String>) {
        self.error_message = error_message;
    }

    pub fn get_cdn_image_uri(&self) -> Option<String> {
        self.cdn_image_uri.clone()
    }

    pub fn set_cdn_image_uri(&mut self, cdn_image_uri: Option<String>) {
        self.cdn_image_uri = cdn_image_uri;
    }

    pub fn get_num_failures(&self) -> i64 {
        self.num_failures
    }

    pub fn increment_num_failures(&mut self) {
        self.num_failures += 1;
    }
}
