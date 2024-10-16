// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, filters::MoveStructTagFilter, traits::Filterable};
use anyhow::Error;
use aptos_protos::transaction::v1::{move_type::Content, Event};
use serde::{Deserialize, Serialize};

/// Example:
/// ```
/// use aptos_transaction_filter::{EventFilterBuilder, MoveStructTagFilterBuilder};
///
/// let move_struct_tag_filter = MoveStructTagFilterBuilder::default()
///   .address("0x0077")
///   .module("roulette")
///   .name("spin")
///   .build()
///   .unwrap();
/// let filter = EventFilterBuilder::default()
///   .struct_type(move_struct_tag_filter)
///   .build()
///   .unwrap();
/// ```
#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(strip_option), default)]
pub struct EventFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(into, strip_option), default)]
    pub data: Option<String>,
    // Only for events that have a struct as their generic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub struct_type: Option<MoveStructTagFilter>,
}

impl Filterable<Event> for EventFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.data.is_none() && self.struct_type.is_none() {
            return Err(Error::msg("At least one of data or struct_type must be set").into());
        };

        self.data.is_valid()?;
        self.struct_type.is_valid()?;
        Ok(())
    }

    #[inline]
    fn is_allowed(&self, item: &Event) -> bool {
        if let Some(struct_type_filter) = &self.struct_type {
            if let Some(Content::Struct(struct_tag)) =
                &item.r#type.as_ref().and_then(|t| t.content.as_ref())
            {
                if !struct_type_filter.is_allowed(struct_tag) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if !self.data.is_allowed(&item.data) {
            return false;
        }

        true
    }
}
