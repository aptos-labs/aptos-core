// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::accept_type::{parse_accept, AcceptType};
use super::{
    build_not_found, ApiTags, BadRequestError, BasicResponse, BasicResponseStatus, InternalError,
    NotFoundError,
};
use super::{BasicErrorWith404, BasicResultWith404};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    Address, AsConverter, IdentifierWrapper, MoveModuleBytecode, MoveStructTag,
    MoveStructTagWrapper, MoveValue, TableItemRequest, TransactionId, U128,
};
use aptos_api_types::{LedgerInfo, MoveResource};
use aptos_state_view::StateView;
use aptos_types::access_path::AccessPath;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::table::TableHandle;
use aptos_vm::data_cache::AsMoveResolver;
use move_deps::move_core_types::language_storage::{ModuleId, ResourceKey, StructTag};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{param::Path, OpenApi};
use std::convert::TryInto;
use std::sync::Arc;
use storage_interface::state_view::DbStateView;

pub struct StateApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl StateApi {
    /// Get specific account resource
    ///
    /// This endpoint returns the resource of a specific type residing at a given
    /// account at a specified ledger version (AKA transaction version). If the
    /// ledger version is not specified in the request, the latest ledger version
    /// is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window (link).
    /// If the requested data has been pruned, the server responds with a 404.
    #[oai(
        path = "/accounts/:address/resource/:resource_type",
        method = "get",
        operation_id = "get_account_resource",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_resource(
        &self,
        accept: Accept,
        address: Path<Address>,
        resource_type: Path<MoveStructTagWrapper>,
        ledger_version: Query<Option<u64>>,
    ) -> BasicResultWith404<MoveResource> {
        fail_point_poem("endpoint_get_account_resource")?;
        let accept_type = parse_accept(&accept)?;
        self.resource(&accept_type, address.0, resource_type.0, ledger_version.0)
    }

    /// Get specific account module
    ///
    /// This endpoint returns the module with a specific name residing at a given
    /// account at a specified ledger version (AKA transaction version). If the
    /// ledger version is not specified in the request, the latest ledger version
    /// is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window (link).
    /// If the requested data has been pruned, the server responds with a 404.
    #[oai(
        path = "/accounts/:address/module/:module_name",
        method = "get",
        operation_id = "get_account_module",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_module(
        &self,
        accept: Accept,
        address: Path<Address>,
        module_name: Path<IdentifierWrapper>,
        ledger_version: Query<Option<u64>>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        fail_point_poem("endpoint_get_account_module")?;
        let accept_type = parse_accept(&accept)?;
        self.module(&accept_type, address.0, module_name.0, ledger_version.0)
    }

    // TODO: This was originally POST to handle the nasty input types.
    /// Get table item
    ///
    /// todo
    #[oai(
        path = "/tables/:table_handle/item",
        method = "post",
        operation_id = "get_table_item",
        tag = "ApiTags::Tables"
    )]
    async fn get_table_item(
        &self,
        accept: Accept,
        // TODO: Cut over to u128 or U128 once https://github.com/poem-web/poem/pull/336 lands.
        table_handle: Path<U128>,
        table_item_request: Json<TableItemRequest>,
        ledger_version: Query<Option<u64>>,
    ) -> BasicResultWith404<MoveValue> {
        // TODO: fail_point could just be middleware.
        fail_point_poem("endpoint_get_table_item")?;
        let accept_type = parse_accept(&accept)?;
        self.table_item(
            &accept_type,
            table_handle.0.into(),
            table_item_request.0,
            ledger_version.0,
        )
    }
}

impl StateApi {
    fn preprocess_request<E: NotFoundError + InternalError>(
        &self,
        requested_ledger_version: Option<u64>,
    ) -> Result<(LedgerInfo, u64, DbStateView), E> {
        let latest_ledger_info = self.context.get_latest_ledger_info_poem()?;
        let ledger_version: u64 =
            requested_ledger_version.unwrap_or_else(|| latest_ledger_info.version());

        if ledger_version > latest_ledger_info.version() {
            return Err(build_not_found(
                "ledger",
                TransactionId::Version(ledger_version),
                latest_ledger_info.version(),
            ));
        }

        let state_view = self.context.state_view_at_version(ledger_version)
            .context(format!("Failed to get state view at version {} even after confirming the ledger has advanced past that version to {}", ledger_version, latest_ledger_info.version()))
            .map_err(E::internal)?;

        Ok((latest_ledger_info, ledger_version, state_view))
    }

    fn resource(
        &self,
        accept_type: &AcceptType,
        address: Address,
        resource_type: MoveStructTagWrapper,
        ledger_version: Option<u64>,
    ) -> BasicResultWith404<MoveResource> {
        let resource_type: MoveStructTag = resource_type.into();
        let resource_type: StructTag = resource_type
            .try_into()
            .context("Failed to parse given resource type")
            .map_err(BasicErrorWith404::bad_request)?;
        let resource_key = ResourceKey::new(address.into(), resource_type.clone());
        let access_path = AccessPath::resource_access_path(resource_key.clone());
        let state_key = StateKey::AccessPath(access_path);
        let (ledger_info, ledger_version, state_view) = self.preprocess_request(ledger_version)?;
        let bytes = state_view
            .get_state_value(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(BasicErrorWith404::internal)?
            .ok_or_else(|| build_not_found("Resource", resource_key, ledger_version))?;

        let resource = state_view
            .as_move_resolver()
            .as_converter()
            .try_into_resource(&resource_type, &bytes)
            .context("Failed to deserialize resource data retrieved from DB")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            resource,
            &ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }

    pub fn module(
        &self,
        accept_type: &AcceptType,
        address: Address,
        name: IdentifierWrapper,
        ledger_version: Option<u64>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        let module_id = ModuleId::new(address.into(), name.into());
        let access_path = AccessPath::code_access_path(module_id.clone());
        let state_key = StateKey::AccessPath(access_path);
        let (ledger_info, ledger_version, state_view) = self.preprocess_request(ledger_version)?;
        let bytes = state_view
            .get_state_value(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(BasicErrorWith404::internal)?
            .ok_or_else(|| build_not_found("Module", module_id, ledger_version))?;

        let module = MoveModuleBytecode::new(bytes)
            .try_parse_abi()
            .context("Failed to parse move module ABI from bytes retrieved from storage")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            module,
            &ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }

    pub fn table_item(
        &self,
        accept_type: &AcceptType,
        table_handle: u128,
        table_item_request: TableItemRequest,
        ledger_version: Option<u64>,
    ) -> BasicResultWith404<MoveValue> {
        let key_type = table_item_request
            .key_type
            .try_into()
            .context("Failed to parse key_type")
            .map_err(BasicErrorWith404::bad_request)?;
        let value_type = table_item_request
            .value_type
            .try_into()
            .context("Failed to parse value_type")
            .map_err(BasicErrorWith404::bad_request)?;
        let key = table_item_request.key;

        let (ledger_info, ledger_version, state_view) = self.preprocess_request(ledger_version)?;

        let resolver = state_view.as_move_resolver();
        let converter = resolver.as_converter();

        let vm_key = converter
            .try_into_vm_value(&key_type, key.clone())
            .map_err(BasicErrorWith404::bad_request)?;
        let raw_key = vm_key
            .undecorate()
            .simple_serialize()
            .ok_or_else(|| BasicErrorWith404::internal_str("Failed to serialize table key"))?;

        let state_key = StateKey::table_item(TableHandle(table_handle), raw_key);
        let bytes = state_view
            .get_state_value(&state_key)
            .context(format!(
                "Failed when trying to retrieve table item from the DB with key: {}",
                key
            ))
            .map_err(BasicErrorWith404::internal)?
            .ok_or_else(|| build_not_found("table handle or item", key, ledger_version))?;

        let move_value = converter
            .try_into_move_value(&value_type, &bytes)
            .context("Failed to deserialize table item retrieved from DB")
            .map_err(BasicErrorWith404::internal)?;

        BasicResponse::try_from_rust_value((
            move_value,
            &ledger_info,
            BasicResponseStatus::Ok,
            accept_type,
        ))
    }
}
