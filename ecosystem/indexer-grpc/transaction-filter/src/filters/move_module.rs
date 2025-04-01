// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable, utils::standardize_address};
use anyhow::anyhow;
use aptos_protos::transaction::v1::MoveStructTag;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

/// Example:
/// ```
/// use aptos_transaction_filter::MoveStructTagFilterBuilder;
///
/// let filter = MoveStructTagFilterBuilder::default()
///   .address("0x0000000000000000000000000000000000000000000000000000000000000004")
///   .module("aptos_token")
///   .name("Token")
///   .build()
///   .unwrap();
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(into, strip_option), default)]
pub struct MoveStructTagFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip)]
    #[builder(setter(skip))]
    standardized_address: OnceCell<Option<String>>,
}

impl MoveStructTagFilter {
    /// Returns a memoized standardized address, if an address is provided.
    fn get_standardized_address(&self) -> &Option<String> {
        self.standardized_address.get_or_init(|| {
            self.address
                .as_ref()
                .map(|address| standardize_address(address))
        })
    }
}

impl From<aptos_protos::indexer::v1::MoveStructTagFilter> for MoveStructTagFilter {
    fn from(proto_filter: aptos_protos::indexer::v1::MoveStructTagFilter) -> Self {
        Self {
            standardized_address: OnceCell::with_value(
                proto_filter
                    .address
                    .as_ref()
                    .map(|address| standardize_address(address)),
            ),
            address: proto_filter.address,
            module: proto_filter.module,
            name: proto_filter.name,
        }
    }
}

impl From<MoveStructTagFilter> for aptos_protos::indexer::v1::MoveStructTagFilter {
    fn from(move_struct_tag_filter: MoveStructTagFilter) -> Self {
        Self {
            address: move_struct_tag_filter.address,
            module: move_struct_tag_filter.module,
            name: move_struct_tag_filter.name,
        }
    }
}

impl Filterable<MoveStructTag> for MoveStructTagFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.address.is_none() && self.module.is_none() && self.name.is_none() {
            return Err(anyhow!("At least one of address, module or name must be set").into());
        };
        Ok(())
    }

    #[inline]
    fn matches(&self, struct_tag: &MoveStructTag) -> bool {
        self.get_standardized_address()
            .matches(&standardize_address(&struct_tag.address))
            && self.module.matches(&struct_tag.module)
            && self.name.matches(&struct_tag.name)
    }
}
