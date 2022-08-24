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
use aptos_api_types::{
    Address, AptosErrorCode, EventKey, IdentifierWrapper, LedgerInfo, MoveStructTag, U64,
};
use aptos_api_types::{AsConverter, VersionedEvent};
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};

pub struct EventsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl EventsApi {
    /// Get events by event key
    ///
    /// This endpoint allows you to get a list of events of a specific type
    /// as identified by its event key, which is a globally unique ID.
    #[oai(
        path = "/events/:event_key",
        method = "get",
        operation_id = "get_events_by_event_key",
        tag = "ApiTags::Events"
    )]
    async fn get_events_by_event_key(
        &self,
        accept_type: AcceptType,
        // TODO: https://github.com/aptos-labs/aptos-core/issues/2278
        event_key: Path<EventKey>,
        start: Query<Option<U64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        fail_point_poem("endpoint_get_events_by_event_key")?;
        self.context
            .check_api_output_enabled("Get events by event key", &accept_type)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);

        // Ensure that account exists
        let account = Account::new(
            self.context.clone(),
            event_key.0 .0.get_creator_address().into(),
            None,
        )?;
        account.account_state()?;
        self.list(account.latest_ledger_info, accept_type, page, event_key.0)
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
        accept_type: AcceptType,
        address: Path<Address>,
        event_handle: Path<MoveStructTag>,
        field_name: Path<IdentifierWrapper>,
        start: Query<Option<U64>>,
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<VersionedEvent>> {
        // TODO: Assert that Event represents u64s as strings.
        fail_point_poem("endpoint_get_events_by_event_handle")?;
        self.context
            .check_api_output_enabled("Get events by event handle", &accept_type)?;
        let page = Page::new(start.0.map(|v| v.0), limit.0);
        let account = Account::new(self.context.clone(), address.0, None)?;
        let key = account
            .find_event_key(event_handle.0, field_name.0.into())?
            .into();
        self.list(account.latest_ledger_info, accept_type, page, key)
    }
}

impl EventsApi {
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
                &event_key.into(),
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
