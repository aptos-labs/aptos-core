// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::nft_metadata_crawler::asset_uploader_request_statuses;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(idempotency_key, application_id, asset_uri))]
#[diesel(table_name = asset_uploader_request_statuses)]
pub struct AssetUploaderRequestStatusesQuery {
    pub idempotency_key: String,
    pub application_id: String,
    pub asset_uri: String,
    pub status_code: i64,
    pub error_messages: Option<Vec<Option<String>>>,
    pub cdn_image_uri: Option<String>,
    pub num_failures: i64,
    pub request_received_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}
