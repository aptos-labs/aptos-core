// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    context::api_spawn_blocking,
    failpoint::fail_point_poem,
    response::{
        api_forbidden, build_not_found, module_not_found, resource_not_found, table_item_not_found,
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        InternalError,
    },
    ApiTags, Context,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    verify_module_identifier, Address, AptosErrorCode, AsConverter, IdentifierWrapper,
    MoveModuleBytecode, MoveResource, MoveStructTag, MoveValue, RawStateValueRequest,
    RawTableItemRequest, TableItemRequest, VerifyInput, VerifyInputWithRecursion, U64,
};
use aptos_types::state_store::{state_key::StateKey, table::TableHandle, TStateView};
use move_core_types::language_storage::StructTag;
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    OpenApi,
};
use std::{convert::TryInto, sync::Arc};

/// API for retrieving individual state
#[derive(Clone)]
pub struct StateApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl StateApi {
    /// Get account resource
    ///
    /// Retrieves an individual resource from a given account and at a specific ledger version. If the
    /// ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/resource/:resource_type",
        method = "get",
        operation_id = "get_account_resource",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_resource(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Name of struct to retrieve e.g. `0x1::account::Account`
        resource_type: Path<MoveStructTag>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveResource> {
        resource_type
            .0
            .verify(0)
            .context("'resource_type' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_account_resource")?;
        self.context
            .check_api_output_enabled("Get account resource", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.resource(
                &accept_type,
                address.0,
                resource_type.0,
                ledger_version.0.map(|inner| inner.0),
            )
        })
        .await
    }

    /// Get account module
    ///
    /// Retrieves an individual module from a given account and at a specific ledger version. If the
    /// ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/module/:module_name",
        method = "get",
        operation_id = "get_account_module",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_module(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Name of module to retrieve e.g. `coin`
        module_name: Path<IdentifierWrapper>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        verify_module_identifier(module_name.0.as_str())
            .context("'module_name' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_account_module")?;
        self.context
            .check_api_output_enabled("Get account module", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.module(&accept_type, address.0, module_name.0, ledger_version.0)
        })
        .await
    }

    /// Get table item
    ///
    /// Get a table item at a specific ledger version from the table identified by {table_handle}
    /// in the path and the "key" (TableItemRequest) provided in the request body.
    ///
    /// This is a POST endpoint because the "key" for requesting a specific
    /// table item (TableItemRequest) could be quite complex, as each of its
    /// fields could themselves be composed of other structs. This makes it
    /// impractical to express using query params, meaning GET isn't an option.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/tables/:table_handle/item",
        method = "post",
        operation_id = "get_table_item",
        tag = "ApiTags::Tables"
    )]
    async fn get_table_item(
        &self,
        accept_type: AcceptType,
        /// Table handle hex encoded 32-byte string
        table_handle: Path<Address>,
        /// Table request detailing the key type, key, and value type
        table_item_request: Json<TableItemRequest>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        table_item_request
            .0
            .verify()
            .context("'table_item_request' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_table_item")?;
        self.context
            .check_api_output_enabled("Get table item", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.table_item(
                &accept_type,
                table_handle.0,
                table_item_request.0,
                ledger_version.0,
            )
        })
        .await
    }

    /// Get raw table item
    ///
    /// Get a table item at a specific ledger version from the table identified by {table_handle}
    /// in the path and the "key" (RawTableItemRequest) provided in the request body.
    ///
    /// The `get_raw_table_item` requires only a serialized key comparing to the full move type information
    /// comparing to the `get_table_item` api, and can only return the query in the bcs format.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/tables/:table_handle/raw_item",
        method = "post",
        operation_id = "get_raw_table_item",
        tag = "ApiTags::Tables"
    )]
    async fn get_raw_table_item(
        &self,
        accept_type: AcceptType,
        /// Table handle hex encoded 32-byte string
        table_handle: Path<Address>,
        /// Table request detailing the key type, key, and value type
        table_item_request: Json<RawTableItemRequest>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        fail_point_poem("endpoint_get_table_item")?;

        if AcceptType::Json == accept_type {
            return Err(api_forbidden(
                "Get raw table item",
                "Only BCS is supported as an AcceptType.",
            ));
        }
        self.context
            .check_api_output_enabled("Get raw table item", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.raw_table_item(
                &accept_type,
                table_handle.0,
                table_item_request.0,
                ledger_version.0,
            )
        })
        .await
    }

    /// Get raw state value.
    ///
    /// Get a state value at a specific ledger version, identified by the key provided
    /// in the request body.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/experimental/state_values/raw",
        method = "post",
        operation_id = "get_raw_state_value",
        tag = "ApiTags::Experimental",
        hidden
    )]
    async fn get_raw_state_value(
        &self,
        accept_type: AcceptType,
        /// Request that carries the state key.
        request: Json<RawStateValueRequest>,
        /// Ledger version at which the value is got.
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        fail_point_poem("endpoint_get_raw_state_value")?;

        if AcceptType::Json == accept_type {
            return Err(api_forbidden(
                "Get raw state value",
                "Only BCS is supported as an AcceptType.",
            ));
        }
        self.context
            .check_api_output_enabled("Get raw state value", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || api.raw_value(&accept_type, request.0, ledger_version.0)).await
    }
}

impl StateApi {
    /// Read a resource at the ledger version
    ///
    /// JSON: Convert to MoveResource
    /// BCS: Leave it encoded as the resource
    fn resource(
        &self,
        accept_type: &AcceptType,
        address: Address,
        resource_type: MoveStructTag,
        ledger_version: Option<u64>,
    ) -> BasicResultWith404<MoveResource> {
        let tag: StructTag = (&resource_type)
            .try_into()
            .context("Failed to parse given resource type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;

        let (ledger_info, ledger_version, state_view) = self.context.state_view(ledger_version)?;
        let bytes = state_view
            .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
            .find_resource(&state_view, address, &tag)
            .context(format!(
                "Failed to query DB to check for {} at {}",
                tag.to_canonical_string(),
                address
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| resource_not_found(address, &tag, ledger_version, &ledger_info))?;

        match accept_type {
            AcceptType::Json => {
                let resource = state_view
                    .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
                    .try_into_resource(&tag, &bytes)
                    .context("Failed to deserialize resource data retrieved from DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((resource, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve the module
    ///
    /// JSON: Parse ABI and bytecode
    /// BCS: Leave bytecode as is BCS encoded
    pub fn module(
        &self,
        accept_type: &AcceptType,
        address: Address,
        name: IdentifierWrapper,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        let state_key = StateKey::module(address.inner(), &name);
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| module_not_found(address, &name, ledger_version, &ledger_info))?;

        match accept_type {
            AcceptType::Json => {
                let module = MoveModuleBytecode::new(bytes.to_vec())
                    .try_parse_abi()
                    .context("Failed to parse move module ABI from bytes retrieved from storage")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((module, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve table item for a specific ledger version
    pub fn table_item(
        &self,
        accept_type: &AcceptType,
        table_handle: Address,
        table_item_request: TableItemRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Parse the key and value types for the table
        let key_type = (&table_item_request.key_type)
            .try_into()
            .context("Failed to parse key_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        let key = table_item_request.key;
        let value_type = (&table_item_request.value_type)
            .try_into()
            .context("Failed to parse value_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;

        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let converter =
            state_view.as_converter(self.context.db.clone(), self.context.indexer_reader.clone());

        // Convert key to lookup version for DB
        let vm_key = converter
            .try_into_vm_value(&key_type, key.clone())
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                )
            })?;
        let raw_key = vm_key.undecorate().simple_serialize().ok_or_else(|| {
            BasicErrorWith404::bad_request_with_code(
                "Failed to serialize table key",
                AptosErrorCode::InvalidInput,
                &ledger_info,
            )
        })?;

        // Retrieve value from the state key
        let state_key = StateKey::table_item(&TableHandle(table_handle.into()), &raw_key);
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!(
                "Failed when trying to retrieve table item from the DB with key: {}",
                key
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                table_item_not_found(table_handle, &key, ledger_version, &ledger_info)
            })?;

        match accept_type {
            AcceptType::Json => {
                let move_value = converter
                    .try_into_move_value(&value_type, &bytes)
                    .context("Failed to deserialize table item retrieved from DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((move_value, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve table item for a specific ledger version
    pub fn raw_table_item(
        &self,
        accept_type: &AcceptType,
        table_handle: Address,
        table_item_request: RawTableItemRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let state_key =
            StateKey::table_item(&TableHandle(table_handle.into()), &table_item_request.key.0);
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!(
                "Failed when trying to retrieve table item from the DB with key: {}",
                table_item_request.key,
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                build_not_found(
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
            AcceptType::Json => Err(api_forbidden(
                "Get raw table item",
                "Please use get table item instead.",
            )),
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve state value for a specific ledger version
    pub fn raw_value(
        &self,
        accept_type: &AcceptType,
        request: RawStateValueRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let state_key = bcs::from_bytes(&request.key.0)
            .context(format!(
                "Failed deserializing state value. key: {}",
                request.key
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;
        let state_value = state_view
            .get_state_value(&state_key)
            .context(format!("Failed fetching state value. key: {}", request.key,))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                build_not_found(
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
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => Err(api_forbidden(
                "Get raw state value",
                "This serves only bytes. Use other APIs for Json.",
            )),
            AcceptType::Bcs => {
                BasicResponse::try_from_encoded((bytes, &ledger_info, BasicResponseStatus::Ok))
            },
        }
    }
}
