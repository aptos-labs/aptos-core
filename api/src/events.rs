// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::accept_type::AcceptType;
use crate::accounts::Account;
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::page::Page;
use crate::response::{
    BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404, InternalError,
};
use crate::ApiTags;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{Address, AptosErrorCode, IdentifierWrapper, LedgerInfo, MoveStructTag, U64};
use aptos_api_types::{AsConverter, VersionedEvent};
use aptos_types::event::EventKey;
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};

pub struct EventsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl EventsApi {
    /// Get events by creation number
    ///
    /// Event streams are globally identifiable by an account `address` and
    /// monotonically increasing `creation_number`, one per event stream
    /// originating from the given account. This API returns events
    /// corresponding to that event stream.
    #[oai(
        path = "/accounts/:address/events/:creation_number",
        method = "get",
        operation_id = "get_events_by_creation_number",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_creation_number(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix. This is should be
        /// the account that published the Move module that defined the event
        /// stream you are trying to read, not any account the event might be
        /// affecting.
        address: Path<Address>,
        /// Creation number corresponding to the event stream originating
        /// from the given account.
        creation_number: Path<U64>,
        /// Starting sequence number of events.
        ///
        /// By default, will retrieve the most recent events
        start: Query<Option<U64>>,
        /// Max number of events to retrieve.
        ///
        /// Mo value defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        fail_point_poem("endpoint_get_events_by_event_key")?;
        self.context
            .check_api_output_enabled("Get events by event key", &accept_type)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);

        // Ensure that account exists
        let account = Account::new(self.context.clone(), address.0, None)?;
        account.account_state()?;
        self.list(
            account.latest_ledger_info,
            accept_type,
            page,
            EventKey::new(creation_number.0 .0, address.0.into()),
        )
    }

    /// Get events by event handle
    ///
    /// This API uses the given account `address`, `event_handle`, and
    /// `field_name` to build a key that globally identify an event stream.
    ///  It then uses this key to return events from that stream.
    #[oai(
        path = "/accounts/:address/events/:event_handle/:field_name",
        method = "get",
        operation_id = "get_events_by_event_handle",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_event_handle(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Name of struct to lookup event handle e.g. `0x1::account::Account`
        event_handle: Path<MoveStructTag>,
        /// Name of field to lookup event handle e.g. `withdraw_events`
        field_name: Path<IdentifierWrapper>,
        /// Starting sequence number of events.
        ///
        /// By default, will retrieve the most recent events
        start: Query<Option<U64>>,
        /// Max number of events to retrieve.
        ///
        /// Mo value defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        // TODO: Assert that Event represents u64s as strings.
        fail_point_poem("endpoint_get_events_by_event_handle")?;
        self.context
            .check_api_output_enabled("Get events by event handle", &accept_type)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);
        let account = Account::new(self.context.clone(), address.0, None)?;
        let key = account.find_event_key(event_handle.0, field_name.0.into())?;
        self.list(account.latest_ledger_info, accept_type, page, key)
    }
}

impl EventsApi {
    /// List events from an [`EventKey`]
    fn list(
        &self,
        latest_ledger_info: LedgerInfo,
        accept_type: AcceptType,
        page: Page,
        event_key: EventKey,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        let ledger_version = latest_ledger_info.version();
        let events = self
            .context
            .get_events(
                &event_key,
                page.start_option(),
                page.limit(&latest_ledger_info)?,
                ledger_version,
            )
            .context(format!("Failed to find events by key {}", event_key))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                let resolver = self.context.move_resolver_poem(&latest_ledger_info)?;
                let events = resolver
                    .as_converter(self.context.db.clone())
                    .try_into_versioned_events(&events)
                    .context("Failed to convert events from storage into response")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &latest_ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((events, &latest_ledger_info, BasicResponseStatus::Ok))
            }
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((events, &latest_ledger_info, BasicResponseStatus::Ok))
            }
        }
    }
}
