// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;
use std::sync::Arc;

use super::accept_type::AcceptType;
use super::page::Page;
use super::{
    response::{AptosInternalResult, AptosResponseResult},
    ApiTags, AptosResponse,
};
use super::{AptosError, AptosErrorCode, AptosErrorResponse};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::format_err;
use aptos_api_types::EventKey;
use aptos_api_types::LedgerInfo;
use aptos_api_types::{AsConverter, Event};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{param::Path, OpenApi};

// TODO: Make a helper that builds an AptosResponse from just an anyhow error,
// that assumes that it's an internal error. We can use .context() add more info.

pub struct EventsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl EventsApi {
    /// Get events by event key
    ///
    /// todo
    #[oai(
        path = "/events/:event_key",
        method = "get",
        operation_id = "get_events_by_event_key",
        tag = "ApiTags::General"
    )]
    async fn get_events_by_event_key(
        &self,
        accept: Accept,
        // TODO: Make this a little smarter, in the spec this just looks like a string.
        // Consider unpacking the inner EventKey type and taking two params, the creation
        // number and the address.
        event_key: Path<EventKey>,
        start: Query<Option<u64>>,
        limit: Query<Option<u16>>,
    ) -> AptosResponseResult<Vec<Event>> {
        fail_point_poem("endpoint_get_events_by_event_key")?;
        let accept_type = AcceptType::try_from(&accept)?;
        let page = Page::new(start.0, limit.0);
        let events = Events::new(self.context.clone(), event_key.0)?;
        events.list(&accept_type, page)
    }
}

struct Events {
    context: Arc<Context>,
    event_key: EventKey,
    latest_ledger_info: LedgerInfo,
}

impl Events {
    pub fn new(context: Arc<Context>, event_key: EventKey) -> AptosInternalResult<Self> {
        let latest_ledger_info = context.get_latest_ledger_info_poem()?;

        Ok(Self {
            context,
            event_key,
            latest_ledger_info,
        })
    }

    pub fn list(self, accept_type: &AcceptType, page: Page) -> AptosResponseResult<Vec<Event>> {
        let contract_events = self
            .context
            .get_events(
                &self.event_key.into(),
                page.start(0, u64::MAX)?,
                page.limit()?,
                self.latest_ledger_info.version(),
            )
            // TODO: Previously this was a 500, but I'm making this a 400. I suspect
            // both could be true depending on the error. Make this more specific.
            .map_err(|e| {
                AptosErrorResponse::BadRequest(Json(
                    AptosError::new(
                        format_err!("Failed to find events by key {}: {}", self.event_key, e)
                            .to_string(),
                    )
                    .error_code(AptosErrorCode::InvalidBcsInStorageError),
                ))
            })?;

        let resolver = self.context.move_resolver_poem()?;
        let events = resolver
            .as_converter()
            .try_into_events(&contract_events)
            .map_err(|e| {
                AptosErrorResponse::InternalServerError(Json(
                    AptosError::new(
                        format_err!("Failed to convert events from storage into response: {}", e)
                            .to_string(),
                    )
                    .error_code(AptosErrorCode::InvalidBcsInStorageError),
                ))
            })?;

        AptosResponse::try_from_rust_value(events, &self.latest_ledger_info, accept_type)
    }
}
