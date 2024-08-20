// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, traits::Filterable};
use anyhow::anyhow;
use aptos_protos::transaction::v1::MoveStructTag;
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
    fn is_allowed(&self, struct_tag: &MoveStructTag) -> bool {
        self.address.is_allowed(&struct_tag.address)
            && self.module.is_allowed(&struct_tag.module)
            && self.name.is_allowed(&struct_tag.name)
    }
}
