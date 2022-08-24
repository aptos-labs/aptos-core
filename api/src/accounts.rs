// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::response::{
    account_not_found, resource_not_found, struct_field_not_found, BadRequestError,
    BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404, InternalError,
};
use crate::ApiTags;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    AccountData, Address, AptosErrorCode, AsConverter, LedgerInfo, MoveModuleBytecode,
    MoveModuleId, MoveResource, MoveStructTag, U64,
};
use aptos_types::access_path::AccessPath;
use aptos_types::account_config::AccountResource;
use aptos_types::account_state::AccountState;
use aptos_types::event::EventHandle;
use aptos_types::event::EventKey;
use aptos_types::state_store::state_key::StateKey;
use move_deps::move_core_types::value::MoveValue;
use move_deps::move_core_types::{
    identifier::Identifier,
    language_storage::{ResourceKey, StructTag},
    move_resource::MoveStructType,
};
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::sync::Arc;

pub struct AccountsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl AccountsApi {
    /// Get account
    ///
    /// Return high level information about an account such as its sequence number.
    #[oai(
        path = "/accounts/:address",
        method = "get",
        operation_id = "get_account",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account(
        &self,
        accept_type: AcceptType,
        address: Path<Address>,
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<AccountData> {
        fail_point_poem("endpoint_get_account")?;
        self.context
            .check_api_output_enabled("Get account", &accept_type)?;
        let account = Account::new(self.context.clone(), address.0, ledger_version.0)?;
        account.account(&accept_type)
    }

    /// Get account resources
    ///
    /// This endpoint returns all account resources at a given address at a
    /// specific ledger version (AKA transaction version). If the ledger
    /// version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window (link).
    /// If the requested data has been pruned, the server responds with a 404.
    #[oai(
        path = "/accounts/:address/resources",
        method = "get",
        operation_id = "get_account_resources",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_resources(
        &self,
        accept_type: AcceptType,
        address: Path<Address>,
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<MoveResource>> {
        fail_point_poem("endpoint_get_account_resources")?;
        self.context
            .check_api_output_enabled("Get account resources", &accept_type)?;
        let account = Account::new(self.context.clone(), address.0, ledger_version.0)?;
        account.resources(&accept_type)
    }

    /// Get account modules
    ///
    /// This endpoint returns all account modules at a given address at a
    /// specific ledger version (AKA transaction version). If the ledger
    /// version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window (link).
    /// If the requested data has been pruned, the server responds with a 404.
    #[oai(
        path = "/accounts/:address/modules",
        method = "get",
        operation_id = "get_account_modules",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_modules(
        &self,
        accept_type: AcceptType,
        address: Path<Address>,
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<MoveModuleBytecode>> {
        fail_point_poem("endpoint_get_account_modules")?;
        self.context
            .check_api_output_enabled("Get account modules", &accept_type)?;
        let account = Account::new(self.context.clone(), address.0, ledger_version.0)?;
        account.modules(&accept_type)
    }
}

pub struct Account {
    context: Arc<Context>,
    address: Address,
    ledger_version: u64,
    pub latest_ledger_info: LedgerInfo,
}

impl Account {
    pub fn new(
        context: Arc<Context>,
        address: Address,
        requested_ledger_version: Option<U64>,
    ) -> Result<Self, BasicErrorWith404> {
        let (latest_ledger_info, requested_ledger_version) = context
            .get_latest_ledger_info_and_verify_lookup_version(
                requested_ledger_version.map(|inner| inner.0),
            )?;

        Ok(Self {
            context,
            address,
            ledger_version: requested_ledger_version,
            latest_ledger_info,
        })
    }

    // These functions map directly to endpoint functions.

    pub fn account(self, accept_type: &AcceptType) -> BasicResultWith404<AccountData> {
        let state_key = StateKey::AccessPath(AccessPath::resource_access_path(ResourceKey::new(
            self.address.into(),
            AccountResource::struct_tag(),
        )));

        let state_value = self.context.get_state_value_poem(
            &state_key,
            self.ledger_version,
            &self.latest_ledger_info,
        )?;

        let state_value = match state_value {
            Some(state_value) => state_value,
            None => {
                return Err(resource_not_found(
                    self.address,
                    &AccountResource::struct_tag(),
                    self.ledger_version,
                    &self.latest_ledger_info,
                ))
            }
        };

        let account_resource: AccountResource = bcs::from_bytes(&state_value)
            .context("Internal error deserializing response from DB")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?;
        let account_data: AccountData = account_resource.into();

        match accept_type {
            AcceptType::Json => BasicResponse::try_from_json((
                account_data,
                &self.latest_ledger_info,
                BasicResponseStatus::Ok,
            )),
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                state_value,
                &self.latest_ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    pub fn resources(self, accept_type: &AcceptType) -> BasicResultWith404<Vec<MoveResource>> {
        let account_state = self.account_state()?;
        let resources = account_state.get_resources();

        match accept_type {
            AcceptType::Json => {
                let move_resolver = self.context.move_resolver_poem(&self.latest_ledger_info)?;
                let converted_resources = move_resolver
                    .as_converter(self.context.db.clone())
                    .try_into_resources(resources)
                    .context("Failed to build move resource response from data in DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &self.latest_ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((
                    converted_resources,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            }
            AcceptType::Bcs => {
                let resources: BTreeMap<StructTag, Vec<u8>> = resources
                    .map(|(key, value)| (key, value.to_vec()))
                    .collect();
                BasicResponse::try_from_bcs((
                    resources,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            }
        }
    }

    pub fn modules(self, accept_type: &AcceptType) -> BasicResultWith404<Vec<MoveModuleBytecode>> {
        let modules = self.account_state()?.into_modules();
        match accept_type {
            AcceptType::Json => {
                let mut converted_modules = Vec::new();
                for (_, module) in modules {
                    converted_modules.push(
                        MoveModuleBytecode::new(module)
                            .try_parse_abi()
                            .context("Failed to parse move module ABI")
                            .map_err(|err| {
                                BasicErrorWith404::internal_with_code(
                                    err,
                                    AptosErrorCode::InternalError,
                                    &self.latest_ledger_info,
                                )
                            })?,
                    );
                }
                BasicResponse::try_from_json((
                    converted_modules,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            }
            AcceptType::Bcs => {
                let modules: BTreeMap<MoveModuleId, Vec<u8>> = modules
                    .map(|(key, value)| (key.into(), value.to_vec()))
                    .collect();
                BasicResponse::try_from_bcs((
                    modules,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
            }
        }
    }

    // Helpers for processing account state.

    pub fn account_state(&self) -> Result<AccountState, BasicErrorWith404> {
        let state = self
            .context
            .get_account_state(self.address.into(), self.ledger_version)
            .context("Failed to read account state at requested version")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?
            .ok_or_else(|| {
                account_not_found(self.address, self.ledger_version, &self.latest_ledger_info)
            })?;

        Ok(state)
    }

    // Events specific stuff.

    pub fn find_event_key(
        &self,
        event_handle: MoveStructTag,
        field_name: Identifier,
    ) -> Result<EventKey, BasicErrorWith404> {
        let struct_tag: StructTag = event_handle
            .try_into()
            .context("Given event handle was invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &self.latest_ledger_info,
                )
            })?;

        let resource = self.find_resource(&struct_tag)?;

        let (_id, value) = resource
            .into_iter()
            .find(|(id, _)| id == &field_name)
            .ok_or_else(|| {
                struct_field_not_found(
                    self.address,
                    &struct_tag,
                    &field_name,
                    self.ledger_version,
                    &self.latest_ledger_info,
                )
            })?;

        // Serialization should not fail, otherwise it's internal bug
        let event_handle_bytes = bcs::to_bytes(&value)
            .context("Failed to serialize event handle from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?;
        // Deserialization may fail because the bytes are not EventHandle struct type.
        let event_handle: EventHandle = bcs::from_bytes(&event_handle_bytes)
            .context(format!(
                "Deserialization error, field({}) type is not a EventHandle struct",
                field_name
            ))
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &self.latest_ledger_info,
                )
            })?;
        Ok(*event_handle.key())
    }

    fn find_resource(
        &self,
        struct_tag: &StructTag,
    ) -> Result<Vec<(Identifier, MoveValue)>, BasicErrorWith404> {
        let account_state = self.account_state()?;
        let (typ, data) = account_state
            .get_resources()
            .find(|(tag, _data)| tag == struct_tag)
            .ok_or_else(|| {
                resource_not_found(
                    self.address,
                    struct_tag,
                    self.ledger_version,
                    &self.latest_ledger_info,
                )
            })?;
        let move_resolver = self.context.move_resolver_poem(&self.latest_ledger_info)?;
        move_resolver
            .as_converter(self.context.db.clone())
            .move_struct_fields(&typ, data)
            .context("Failed to convert move structs from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })
    }
}
