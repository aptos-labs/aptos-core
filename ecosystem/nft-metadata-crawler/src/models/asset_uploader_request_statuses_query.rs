// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::nft_metadata_crawler::asset_uploader_request_statuses;
use diesel::prelude::*;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(
    Clone,
    Debug,
    Deserialize,
    FieldCount,
    Identifiable,
    Queryable,
    Serialize,
    Hash,
    PartialEq,
    Eq,
    PartialOrd,
    Ord, // TODO: Custom Ord implementation for fairness
)]
#[diesel(primary_key(request_id, asset_uri))]
#[diesel(table_name = asset_uploader_request_statuses)]
pub struct AssetUploaderRequestStatusesQuery {
    pub request_id: Uuid,
    pub asset_uri: String,
    pub application_id: Uuid,
    pub status_code: i64,
    pub error_message: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub num_failures: i64,
    pub request_received_at: chrono::NaiveDateTime,
    pub inserted_at: chrono::NaiveDateTime,
}
