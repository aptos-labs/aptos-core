// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    page::Page,
    response_axum::{AptosErrorResponse, AptosResponse},
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{AptosErrorCode, AsConverter, LedgerInfo, VersionedEvent};
use aptos_types::event::EventKey;
use std::sync::Arc;

/// Framework-agnostic business logic for listing events.
pub fn list_events_inner(
    context: &Arc<Context>,
    latest_ledger_info: LedgerInfo,
    accept_type: AcceptType,
    page: Page,
    event_key: EventKey,
) -> Result<AptosResponse<Vec<VersionedEvent>>, AptosErrorResponse> {
    let ledger_version = latest_ledger_info.version();
    let events = context
        .get_events(
            &event_key,
            page.start_option(),
            page.limit::<AptosErrorResponse>(&latest_ledger_info)?,
            ledger_version,
        )
        .context(format!("Failed to find events by key {}", event_key))
        .map_err(|err| {
            AptosErrorResponse::internal(
                err,
                AptosErrorCode::InternalError,
                Some(&latest_ledger_info),
            )
        })?;

    match accept_type {
        AcceptType::Json => {
            let events = context
                .latest_state_view_poem::<AptosErrorResponse>(&latest_ledger_info)?
                .as_converter(context.db.clone(), context.indexer_reader.clone())
                .try_into_versioned_events(&events)
                .context("Failed to convert events from storage into response")
                .map_err(|err| {
                    AptosErrorResponse::internal(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&latest_ledger_info),
                    )
                })?;

            AptosResponse::try_from_json(events, &latest_ledger_info)
        },
        AcceptType::Bcs => {
            AptosResponse::<Vec<VersionedEvent>>::try_from_bcs(events, &latest_ledger_info)
        },
    }
}
