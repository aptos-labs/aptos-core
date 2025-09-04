// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    accounts::Account,
    context::{api_spawn_blocking, Context},
    failpoint::fail_point_poem,
    page::Page,
    response::{
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        InternalError,
    },
    ApiTags,
};
use anyhow::Context as AnyhowContext;
use velor_api_types::{
    verify_field_identifier, Address, VelorErrorCode, AsConverter, IdentifierWrapper, LedgerInfo,
    MoveStructTag, VerifyInputWithRecursion, VersionedEvent, U64,
};
use velor_types::event::EventKey;
use poem_openapi::{
    param::{Path, Query},
    OpenApi,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct EventsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl EventsApi {
    /// Get events by creation number
    ///
    /// Event types are globally identifiable by an account `address` and
    /// monotonically increasing `creation_number`, one per event type emitted
    /// to the given account. This API returns events corresponding to that
    /// that event type.
    #[oai(
        path = "/accounts/:address/events/:creation_number",
        method = "get",
        operation_id = "get_events_by_creation_number",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_creation_number(
        &self,
        accept_type: AcceptType,
        /// Hex-encoded 32 byte Velor account, with or without a `0x` prefix, for
        /// which events are queried. This refers to the account that events were
        /// emitted to, not the account hosting the move module that emits that
        /// event type.
        address: Path<Address>,
        /// Creation number corresponding to the event stream originating
        /// from the given account.
        creation_number: Path<U64>,
        /// Starting sequence number of events.
        ///
        /// If unspecified, by default will retrieve the most recent events
        start: Query<Option<U64>>,
        /// Max number of events to retrieve.
        ///
        /// If unspecified, defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        fail_point_poem("endpoint_get_events_by_event_key")?;
        self.context
            .check_api_output_enabled("Get events by event key", &accept_type)?;
        let page = Page::new(
            start.0.map(|v| v.0),
            limit.0,
            self.context.max_events_page_size(),
        );

        // Ensure that account exists
        let api = self.clone();
        api_spawn_blocking(move || {
            let account = Account::new(api.context.clone(), address.0, None, None, None)?;
            api.list(
                account.latest_ledger_info,
                accept_type,
                page,
                EventKey::new(creation_number.0 .0, address.0.into()),
            )
        })
        .await
    }

    /// Get events by event handle
    ///
    /// This API uses the given account `address`, `eventHandle`, and `fieldName`
    /// to build a key that can globally identify an event types. It then uses this
    /// key to return events emitted to the given account matching that event type.
    #[oai(
        path = "/accounts/:address/events/:event_handle/:field_name",
        method = "get",
        operation_id = "get_events_by_event_handle",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_event_handle(
        &self,
        accept_type: AcceptType,
        /// Hex-encoded 32 byte Velor account, with or without a `0x` prefix, for
        /// which events are queried. This refers to the account that events were
        /// emitted to, not the account hosting the move module that emits that
        /// event type.
        address: Path<Address>,
        /// Name of struct to lookup event handle e.g. `0x1::account::Account`
        event_handle: Path<MoveStructTag>,
        /// Name of field to lookup event handle e.g. `withdraw_events`
        field_name: Path<IdentifierWrapper>,
        /// Starting sequence number of events.
        ///
        /// If unspecified, by default will retrieve the most recent
        start: Query<Option<U64>>,
        /// Max number of events to retrieve.
        ///
        /// If unspecified, defaults to default page size
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        event_handle
            .0
            .verify(0)
            .context("'event_handle' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, VelorErrorCode::InvalidInput)
            })?;
        verify_field_identifier(field_name.as_str())
            .context("'field_name' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, VelorErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_events_by_event_handle")?;
        self.context
            .check_api_output_enabled("Get events by event handle", &accept_type)?;
        let page = Page::new(
            start.0.map(|v| v.0),
            limit.0,
            self.context.max_events_page_size(),
        );

        let api = self.clone();
        api_spawn_blocking(move || {
            let account = Account::new(api.context.clone(), address.0, None, None, None)?;
            let key = account.find_event_key(event_handle.0, field_name.0.into())?;
            api.list(account.latest_ledger_info, accept_type, page, key)
        })
        .await
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
                    VelorErrorCode::InternalError,
                    &latest_ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                let events = self
                    .context
                    .latest_state_view_poem(&latest_ledger_info)?
                    .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                    .try_into_versioned_events(&events)
                    .context("Failed to convert events from storage into response")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            VelorErrorCode::InternalError,
                            &latest_ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((events, &latest_ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => {
                BasicResponse::try_from_bcs((events, &latest_ledger_info, BasicResponseStatus::Ok))
            },
        }
    }
}
