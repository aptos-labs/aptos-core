// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::accept_type::{parse_accept, AcceptType};
use super::accounts::Account;
use super::page::Page;
use super::{
    ApiTags, BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus,
    BasicResultWith404, InternalError,
};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{Address, EventKey, IdentifierWrapper, MoveStructTagWrapper, U64};
use aptos_api_types::{AsConverter, Event};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};

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
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_event_key(
        &self,
        accept: Accept,
        // TODO: Make this a little smarter, in the spec this just looks like a string.
        // Consider unpacking the inner EventKey type and taking two params, the creation
        // number and the address.
        event_key: Path<EventKey>,
        start: Query<Option<U64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Event>> {
        fail_point_poem("endpoint_get_events_by_event_key")?;
        let accept_type = parse_accept(&accept)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);
        self.list(&accept_type, page, event_key.0)
    }

    /// Get events by event handle
    ///
    /// This API extracts event key from the account resource identified
    /// by the `event_handle_struct` and `field_name`, then returns
    /// events identified by the event key.
    #[oai(
        path = "/accounts/:address/events/:event_handle/:field_name",
        method = "get",
        operation_id = "get_events_by_event_handle",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_event_handle(
        &self,
        accept: Accept,
        address: Path<Address>,
        event_handle: Path<MoveStructTagWrapper>,
        field_name: Path<IdentifierWrapper>,
        start: Query<Option<U64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<Event>> {
        // TODO: Assert that Event represents u64s as strings.
        fail_point_poem("endpoint_get_events_by_event_handle")?;
        let accept_type = parse_accept(&accept)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);
        let account = Account::new(self.context.clone(), address.0, None)?;
        let key = account
            .find_event_key(event_handle.0.into(), field_name.0.into())?
            .into();
        self.list(&accept_type, page, key)
    }
}

impl EventsApi {
    fn list(
        &self,
        accept_type: &AcceptType,
        page: Page,
        event_key: EventKey,
    ) -> BasicResultWith404<Vec<Event>> {
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        let contract_events = self
            .context
            .get_events(
                &event_key.into(),
                page.start(0, u64::MAX)?,
                page.limit()?,
                latest_ledger_info.version(),
            )
            // TODO: Previously this was a 500, but I'm making this a 400. I suspect
            // both could be true depending on the error. Make this more specific.
            .context(format!("Failed to find events by key {}", event_key))
            .map_err(BasicErrorWith404::bad_request)?;

        let resolver = self.context.move_resolver_poem()?;
        let events = resolver
            .as_converter()
            .try_into_events(&contract_events)
            .context("Failed to convert events from storage into response {}")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            events,
            &latest_ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }
}
