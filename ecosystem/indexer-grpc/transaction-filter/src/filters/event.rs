// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{errors::FilterError, filters::MoveStructTagFilter, traits::Filterable};
use anyhow::Error;
use velor_protos::transaction::v1::{move_type::Content, Event};
use derivative::Derivative;
use memchr::memmem::Finder;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

/// Example:
/// ```
/// use velor_transaction_filter::{EventFilterBuilder, MoveStructTagFilterBuilder};
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
#[derive(Clone, Default, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq)]
#[serde(deny_unknown_fields)]
#[derive(derive_builder::Builder)]
#[builder(setter(strip_option), default)]
pub struct EventFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(setter(into, strip_option), default)]
    pub data_substring_filter: Option<String>,
    // Only for events that have a struct as their generic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub struct_type: Option<MoveStructTagFilter>,
    #[serde(skip)]
    #[derivative(PartialEq = "ignore")]
    data_substring_finder: OnceCell<Finder<'static>>,
}

impl From<velor_protos::indexer::v1::EventFilter> for EventFilter {
    fn from(proto_filter: velor_protos::indexer::v1::EventFilter) -> Self {
        Self {
            data_substring_filter: proto_filter.data_substring_filter,
            struct_type: proto_filter.struct_type.map(|f| f.into()),
            data_substring_finder: OnceCell::new(),
        }
    }
}

impl From<EventFilter> for velor_protos::indexer::v1::EventFilter {
    fn from(event_filter: EventFilter) -> Self {
        Self {
            struct_type: event_filter.struct_type.map(Into::into),
            data_substring_filter: event_filter.data_substring_filter,
        }
    }
}

impl Filterable<Event> for EventFilter {
    #[inline]
    fn validate_state(&self) -> Result<(), FilterError> {
        if self.data_substring_filter.is_none() && self.struct_type.is_none() {
            return Err(Error::msg("At least one of data or struct_type must be set").into());
        };

        self.data_substring_filter.is_valid()?;
        self.struct_type.is_valid()?;
        Ok(())
    }

    #[inline]
    fn matches(&self, item: &Event) -> bool {
        if let Some(struct_type_filter) = &self.struct_type {
            if let Some(Content::Struct(struct_tag)) =
                &item.r#type.as_ref().and_then(|t| t.content.as_ref())
            {
                if !struct_type_filter.matches(struct_tag) {
                    return false;
                }
            } else {
                return false;
            }
        }

        if let Some(data_substring_filter) = self.data_substring_filter.as_ref() {
            let finder = self
                .data_substring_finder
                .get_or_init(|| Finder::new(data_substring_filter).into_owned());
            if finder.find(item.data.as_bytes()).is_none() {
                return false;
            }
        }

        true
    }
}
