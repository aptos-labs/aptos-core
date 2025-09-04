// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    asset_uploader::api::IdempotencyTuple,
    models::asset_uploader_request_statuses_query::AssetUploaderRequestStatusesQuery,
    schema::nft_metadata_crawler::asset_uploader_request_statuses,
};
use axum::http::StatusCode;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(
    Clone,
    Debug,
    Deserialize,
    FieldCount,
    Identifiable,
    Insertable,
    Serialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord, // TODO: Custom Ord implementation for fairness
)]
#[diesel(primary_key(idempotency_key, application_id, asset_uri))]
#[diesel(table_name = asset_uploader_request_statuses)]
pub struct AssetUploaderRequestStatuses {
    pub idempotency_key: String,
    pub application_id: String,
    pub asset_uri: String,
    pub status_code: i64,
    pub error_messages: Option<Vec<Option<String>>>,
    pub cdn_image_uri: Option<String>,
    pub num_failures: i64,
}

impl Hash for AssetUploaderRequestStatuses {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idempotency_key.hash(state);
        self.application_id.hash(state);
        self.asset_uri.hash(state);
    }
}

impl AssetUploaderRequestStatuses {
    pub fn new(idempotency_tuple: &IdempotencyTuple, asset_uri: &str) -> Self {
        Self {
            idempotency_key: idempotency_tuple.idempotency_key.to_string(),
            application_id: idempotency_tuple.application_id.to_string(),
            asset_uri: asset_uri.to_string(),
            status_code: StatusCode::ACCEPTED.as_u16() as i64,
            error_messages: None,
            cdn_image_uri: None,
            num_failures: 0,
        }
    }

    pub fn new_completed(
        idempotency_tuple: &IdempotencyTuple,
        asset_uri: &str,
        cdn_image_uri: &str,
    ) -> Self {
        Self {
            idempotency_key: idempotency_tuple.idempotency_key.to_string(),
            application_id: idempotency_tuple.application_id.to_string(),
            asset_uri: asset_uri.to_string(),
            status_code: StatusCode::OK.as_u16() as i64,
            error_messages: None,
            cdn_image_uri: Some(cdn_image_uri.to_string()),
            num_failures: 0,
        }
    }
}

impl From<&AssetUploaderRequestStatusesQuery> for AssetUploaderRequestStatuses {
    fn from(query: &AssetUploaderRequestStatusesQuery) -> Self {
        Self {
            idempotency_key: query.idempotency_key.clone(),
            application_id: query.application_id.clone(),
            asset_uri: query.asset_uri.clone(),
            status_code: query.status_code,
            error_messages: query.error_messages.clone(),
            cdn_image_uri: query.cdn_image_uri.clone(),
            num_failures: query.num_failures,
        }
    }
}
