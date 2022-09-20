// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::response::{module_not_found, resource_not_found, table_item_not_found, StdApiError};
use crate::{
    accept_type::AcceptType,
    failpoint::fail_point_poem,
    response::{
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        InternalError,
    },
    ApiTags, Context,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    verify_module_identifier, Address, AptosErrorCode, AsConverter, IdentifierWrapper, LedgerInfo,
    MoveModuleBytecode, MoveResource, MoveStructTag, MoveValue, TableItemRequest, VerifyInput,
    VerifyInputWithRecursion, U64,
};
use aptos_state_view::StateView;
use aptos_types::{
    access_path::AccessPath,
    state_store::{state_key::StateKey, table::TableHandle},
};
use aptos_vm::data_cache::AsMoveResolver;
use move_deps::move_core_types::language_storage::{ModuleId, ResourceKey, StructTag};
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    OpenApi,
};
use std::{convert::TryInto, sync::Arc};
use storage_interface::state_view::DbStateView;

/// API for retrieving individual state
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
        self.resource(
            &accept_type,
            address.0,
            resource_type.0,
            ledger_version.0.map(|inner| inner.0),
        )
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
        self.module(&accept_type, address.0, module_name.0, ledger_version.0)
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
        self.table_item(
            &accept_type,
            table_handle.0,
            table_item_request.0,
            ledger_version.0,
        )
    }
}

impl StateApi {
    /// Retrieve state at the requested ledger version
    fn preprocess_request<E: StdApiError>(
        &self,
        requested_ledger_version: Option<u64>,
    ) -> Result<(LedgerInfo, u64, DbStateView), E> {
        let (latest_ledger_info, requested_ledger_version) = self
            .context
            .get_latest_ledger_info_and_verify_lookup_version(requested_ledger_version)?;

        let state_view = self
            .context
            .state_view_at_version(requested_ledger_version)
            .map_err(|err| {
                E::internal_with_code(err, AptosErrorCode::InternalError, &latest_ledger_info)
            })?;

        Ok((latest_ledger_info, requested_ledger_version, state_view))
    }

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
        let resource_type: StructTag = resource_type
            .try_into()
            .context("Failed to parse given resource type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        let (ledger_info, ledger_version, state_view) = self.preprocess_request(ledger_version)?;
        let resource_key = ResourceKey::new(address.into(), resource_type.clone());
        let access_path = AccessPath::resource_access_path(resource_key);
        let state_key = StateKey::AccessPath(access_path);
        let bytes = state_view
            .get_state_value(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                resource_not_found(address, &resource_type, ledger_version, &ledger_info)
            })?;

        match accept_type {
            AcceptType::Json => {
                let resource = state_view
                    .as_move_resolver()
                    .as_converter(self.context.db.clone())
                    .try_into_resource(&resource_type, &bytes)
                    .context("Failed to deserialize resource data retrieved from DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((resource, &ledger_info, BasicResponseStatus::Ok))
            }
            AcceptType::Bcs => {
                BasicResponse::try_from_encoded((bytes, &ledger_info, BasicResponseStatus::Ok))
            }
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
        let module_id = ModuleId::new(address.into(), name.into());
        let access_path = AccessPath::code_access_path(module_id.clone());
        let state_key = StateKey::AccessPath(access_path);
        let (ledger_info, ledger_version, state_view) =
            self.preprocess_request(ledger_version.map(|inner| inner.0))?;
        let bytes = state_view
            .get_state_value(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                module_not_found(address, module_id.name(), ledger_version, &ledger_info)
            })?;

        match accept_type {
            AcceptType::Json => {
                let module = MoveModuleBytecode::new(bytes)
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
            }
            AcceptType::Bcs => {
                BasicResponse::try_from_encoded((bytes, &ledger_info, BasicResponseStatus::Ok))
            }
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
        let key_type = table_item_request
            .key_type
            .try_into()
            .context("Failed to parse key_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        let key = table_item_request.key;
        let value_type = table_item_request
            .value_type
            .try_into()
            .context("Failed to parse value_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;

        // Retrieve local state
        let (ledger_info, ledger_version, state_view) =
            self.preprocess_request(ledger_version.map(|inner| inner.0))?;

        let resolver = state_view.as_move_resolver();
        let converter = resolver.as_converter(self.context.db.clone());

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
        let state_key = StateKey::table_item(TableHandle(table_handle.into()), raw_key);
        let bytes = state_view
            .get_state_value(&state_key)
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
            }
            AcceptType::Bcs => {
                BasicResponse::try_from_encoded((bytes, &ledger_info, BasicResponseStatus::Ok))
            }
        }
    }
}
