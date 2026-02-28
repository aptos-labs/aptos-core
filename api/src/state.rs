// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    response::{build_not_found, module_not_found, resource_not_found, table_item_not_found},
    response_axum::{AptosErrorResponse, AptosResponse},
    Context,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    Address, AptosErrorCode, AsConverter, IdentifierWrapper, MoveModuleBytecode, MoveResource,
    MoveStructTag, MoveValue, RawStateValueRequest, RawTableItemRequest, TableItemRequest,
};
use aptos_types::state_store::{state_key::StateKey, table::TableHandle, TStateView};
use move_core_types::language_storage::StructTag;
use std::{convert::TryInto, sync::Arc};

/// Framework-agnostic business logic for the get account resource endpoint.
pub fn resource_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    address: Address,
    resource_type: MoveStructTag,
    ledger_version: Option<u64>,
) -> Result<AptosResponse<MoveResource>, AptosErrorResponse> {
    let tag: StructTag = (&resource_type)
        .try_into()
        .context("Failed to parse given resource type")
        .map_err(|err| AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, None))?;

    let (ledger_info, ledger_version, state_view) =
        context.state_view::<AptosErrorResponse>(ledger_version)?;
    let bytes = state_view
        .as_converter(context.db.clone(), context.indexer_reader.clone())
        .find_resource(&state_view, address, &tag)
        .context(format!(
            "Failed to query DB to check for {} at {}",
            tag.to_canonical_string(),
            address
        ))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?
        .ok_or_else(|| {
            resource_not_found::<AptosErrorResponse>(address, &tag, ledger_version, &ledger_info)
        })?;

    match accept_type {
        AcceptType::Json => {
            let resource = state_view
                .as_converter(context.db.clone(), context.indexer_reader.clone())
                .try_into_resource(&tag, &bytes)
                .context("Failed to deserialize resource data retrieved from DB")
                .map_err(|err| {
                    AptosErrorResponse::internal(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&ledger_info),
                    )
                })?;

            AptosResponse::try_from_json(resource, &ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_encoded(bytes.to_vec(), &ledger_info),
    }
}

/// Framework-agnostic business logic for the get account module endpoint.
pub fn module_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    address: Address,
    name: IdentifierWrapper,
    ledger_version: Option<u64>,
) -> Result<AptosResponse<MoveModuleBytecode>, AptosErrorResponse> {
    let state_key = StateKey::module(address.inner(), &name);
    let (ledger_info, ledger_version, state_view) =
        context.state_view::<AptosErrorResponse>(ledger_version)?;
    let bytes = state_view
        .get_state_value_bytes(&state_key)
        .context(format!("Failed to query DB to check for {:?}", state_key))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?
        .ok_or_else(|| {
            module_not_found::<AptosErrorResponse>(address, &name, ledger_version, &ledger_info)
        })?;

    match accept_type {
        AcceptType::Json => {
            let module = MoveModuleBytecode::new(bytes.to_vec())
                .try_parse_abi()
                .context("Failed to parse move module ABI from bytes retrieved from storage")
                .map_err(|err| {
                    AptosErrorResponse::internal(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&ledger_info),
                    )
                })?;

            AptosResponse::try_from_json(module, &ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_encoded(bytes.to_vec(), &ledger_info),
    }
}

/// Framework-agnostic business logic for the get table item endpoint.
pub fn table_item_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    table_handle: Address,
    table_item_request: TableItemRequest,
    ledger_version: Option<u64>,
) -> Result<AptosResponse<MoveValue>, AptosErrorResponse> {
    let key_type = (&table_item_request.key_type)
        .try_into()
        .context("Failed to parse key_type")
        .map_err(|err| AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, None))?;
    let key = table_item_request.key;
    let value_type = (&table_item_request.value_type)
        .try_into()
        .context("Failed to parse value_type")
        .map_err(|err| AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, None))?;

    let (ledger_info, ledger_version, state_view) =
        context.state_view::<AptosErrorResponse>(ledger_version)?;

    let converter = state_view.as_converter(context.db.clone(), context.indexer_reader.clone());

    let vm_key = converter
        .try_into_vm_value(&key_type, key.clone())
        .map_err(|err| {
            AptosErrorResponse::bad_request(err, AptosErrorCode::InvalidInput, Some(&ledger_info))
        })?;
    let raw_key = vm_key.undecorate().simple_serialize().ok_or_else(|| {
        AptosErrorResponse::bad_request(
            "Failed to serialize table key",
            AptosErrorCode::InvalidInput,
            Some(&ledger_info),
        )
    })?;

    let state_key = StateKey::table_item(&TableHandle(table_handle.into()), &raw_key);
    let bytes = state_view
        .get_state_value_bytes(&state_key)
        .context(format!(
            "Failed when trying to retrieve table item from the DB with key: {}",
            key
        ))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?
        .ok_or_else(|| {
            table_item_not_found::<AptosErrorResponse>(
                table_handle,
                &key,
                ledger_version,
                &ledger_info,
            )
        })?;

    match accept_type {
        AcceptType::Json => {
            let move_value = converter
                .try_into_move_value(&value_type, &bytes)
                .context("Failed to deserialize table item retrieved from DB")
                .map_err(|err| {
                    AptosErrorResponse::internal(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&ledger_info),
                    )
                })?;

            AptosResponse::try_from_json(move_value, &ledger_info)
        },
        AcceptType::Bcs => AptosResponse::try_from_encoded(bytes.to_vec(), &ledger_info),
    }
}

/// Framework-agnostic business logic for the get raw table item endpoint.
pub fn raw_table_item_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    table_handle: Address,
    table_item_request: RawTableItemRequest,
    ledger_version: Option<u64>,
) -> Result<AptosResponse<MoveValue>, AptosErrorResponse> {
    let (ledger_info, ledger_version, state_view) =
        context.state_view::<AptosErrorResponse>(ledger_version)?;

    let state_key =
        StateKey::table_item(&TableHandle(table_handle.into()), &table_item_request.key.0);
    let bytes = state_view
        .get_state_value_bytes(&state_key)
        .context(format!(
            "Failed when trying to retrieve table item from the DB with key: {}",
            table_item_request.key,
        ))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?
        .ok_or_else(|| {
            build_not_found::<_, AptosErrorResponse>(
                "Table Item",
                format!(
                    "Table handle({}), Table key({}) and Ledger version({})",
                    table_handle, table_item_request.key, ledger_version
                ),
                AptosErrorCode::TableItemNotFound,
                &ledger_info,
            )
        })?;

    match accept_type {
        AcceptType::Json => Err(crate::response_axum::api_forbidden(
            "Get raw table item",
            "Please use get table item instead.",
        )),
        AcceptType::Bcs => AptosResponse::try_from_encoded(bytes.to_vec(), &ledger_info),
    }
}

/// Framework-agnostic business logic for the get raw state value endpoint.
pub fn raw_value_inner(
    context: &Arc<Context>,
    accept_type: &AcceptType,
    request: RawStateValueRequest,
    ledger_version: Option<u64>,
) -> Result<AptosResponse<MoveValue>, AptosErrorResponse> {
    let (ledger_info, ledger_version, state_view) =
        context.state_view::<AptosErrorResponse>(ledger_version)?;

    let state_key = bcs::from_bytes(&request.key.0)
        .context(format!(
            "Failed deserializing state value. key: {}",
            request.key
        ))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?;
    let state_value = state_view
        .get_state_value(&state_key)
        .context(format!("Failed fetching state value. key: {}", request.key,))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?
        .ok_or_else(|| {
            build_not_found::<_, AptosErrorResponse>(
                "Raw State Value",
                format!(
                    "StateKey({}) and Ledger version({})",
                    request.key, ledger_version
                ),
                AptosErrorCode::StateValueNotFound,
                &ledger_info,
            )
        })?;
    let bytes = bcs::to_bytes(&state_value)
        .context(format!(
            "Failed serializing state value. key: {}",
            request.key
        ))
        .map_err(|err| {
            AptosErrorResponse::internal(err, AptosErrorCode::InternalError, Some(&ledger_info))
        })?;

    match accept_type {
        AcceptType::Json => Err(crate::response_axum::api_forbidden(
            "Get raw state value",
            "This serves only bytes. Use other APIs for Json.",
        )),
        AcceptType::Bcs => AptosResponse::try_from_encoded(bytes, &ledger_info),
    }
}
