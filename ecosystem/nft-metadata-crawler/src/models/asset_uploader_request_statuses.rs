// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery,
    schema::nft_metadata_crawler::asset_uploader_request_statuses,
};
use axum::http::StatusCode;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(request_id, asset_uri))]
#[diesel(table_name = asset_uploader_request_statuses)]
pub struct AssetUploaderRequestStatuses {
    pub request_id: Uuid,
    pub asset_uri: String,
    pub application_id: Uuid,
    pub status_code: i64,
    pub error_message: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub num_failures: i64,
}

impl AssetUploaderRequestStatuses {
    pub fn new(request_id: Uuid, asset_uri: &str, application_id: Uuid) -> Self {
        Self {
            request_id,
            asset_uri: asset_uri.to_string(),
            application_id,
            status_code: StatusCode::ACCEPTED.as_u16() as i64,
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
            status_code: StatusCode::OK.as_u16() as i64,
            error_message: None,
            cdn_image_uri: Some(cdn_image_uri.to_string()),
            num_failures: 0,
        }
    }
}

impl From<&AssetUploaderRequestStatusesQuery> for AssetUploaderRequestStatuses {
    fn from(query: &AssetUploaderRequestStatusesQuery) -> Self {
        Self {
            request_id: query.request_id,
            asset_uri: query.asset_uri.clone(),
            application_id: query.application_id,
            status_code: query.status_code,
            error_message: query.error_message.clone(),
            cdn_image_uri: query.cdn_image_uri.clone(),
            num_failures: query.num_failures,
        }
    }
}
