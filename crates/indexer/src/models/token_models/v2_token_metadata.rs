// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// This is required because a diesel macro makes clippy sad
#![allow(clippy::extra_unused_lifetimes)]
#![allow(clippy::unused_unit)]

use super::{token_utils::NAME_LENGTH, v2_token_utils::TokenV2AggregatedDataMapping};
use crate::{
    models::move_resources::MoveResource,
    schema::current_token_v2_metadata,
    util::{standardize_address, truncate_str},
};
use anyhow::Context;
use velor_api_types::WriteResource;
use field_count::FieldCount;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// PK of current_objects, i.e. object_address, resource_type
pub type CurrentTokenV2MetadataPK = (String, String);

#[derive(Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(object_address, resource_type))]
#[diesel(table_name = current_token_v2_metadata)]
pub struct CurrentTokenV2Metadata {
    pub object_address: String,
    pub resource_type: String,
    pub data: Value,
    pub state_key_hash: String,
    pub last_transaction_version: i64,
}

impl CurrentTokenV2Metadata {
    /// Parsing unknown resources with 0x4::token::Token
    pub fn from_write_resource(
        write_resource: &WriteResource,
        txn_version: i64,
        token_v2_metadata: &TokenV2AggregatedDataMapping,
    ) -> anyhow::Result<Option<Self>> {
        let object_address = standardize_address(&write_resource.address.to_string());
        if let Some(metadata) = token_v2_metadata.get(&object_address) {
            // checking if token_v2
            if metadata.token.is_some() {
                let resource_type_addr = write_resource.data.typ.address.to_string();
                if matches!(resource_type_addr.as_str(), "0x1" | "0x3" | "0x4") {
                    return Ok(None);
                }

                let resource = MoveResource::from_write_resource(write_resource, 0, txn_version, 0);

                let state_key_hash = metadata.object.get_state_key_hash();
                if state_key_hash != resource.state_key_hash {
                    return Ok(None);
                }

                let resource_type = truncate_str(&resource.type_, NAME_LENGTH);
                return Ok(Some(CurrentTokenV2Metadata {
                    object_address,
                    resource_type,
                    data: resource
                        .data
                        .context("data must be present in write resource")?,
                    state_key_hash: resource.state_key_hash,
                    last_transaction_version: txn_version,
                }));
            }
        }
        Ok(None)
    }
}
