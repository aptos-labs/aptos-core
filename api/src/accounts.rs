// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::accept_type::AcceptType;
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use crate::page::determine_limit;
use crate::response::{
    account_not_found, resource_not_found, struct_field_not_found, BadRequestError,
    BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404, InternalError,
};
use crate::ApiTags;
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    AccountData, Address, AptosErrorCode, AsConverter, LedgerInfo, MoveModuleBytecode,
    MoveModuleId, MoveResource, MoveStructTag, StateKeyWrapper, U64,
};
use aptos_types::access_path::AccessPath;
use aptos_types::account_config::AccountResource;
use aptos_types::event::EventHandle;
use aptos_types::event::EventKey;
use aptos_types::state_store::state_key::StateKey;
use move_core_types::value::MoveValue;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ResourceKey, StructTag},
    move_resource::MoveStructType,
};
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::sync::Arc;

/// API for accounts, their associated resources, and modules
pub struct AccountsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl AccountsApi {
    /// Get account
    ///
    /// Return the authentication key and the sequence number for an account
    /// address. Optionally, a ledger version can be specified. If the ledger
    /// version is not specified in the request, the latest ledger version is used.
    #[oai(
        path = "/accounts/:address",
        method = "get",
        operation_id = "get_account",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<AccountData> {
        fail_point_poem("endpoint_get_account")?;
        self.context
            .check_api_output_enabled("Get account", &accept_type)?;
        let account = Account::new(
            self.context.clone(),
            address.0,
            ledger_version.0,
            None,
            None,
        )?;
        account.account(&accept_type)
    }

    /// Get account resources
    ///
    /// Retrieves all account resources for a given account and a specific ledger version.  If the
    /// ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/resources",
        method = "get",
        operation_id = "get_account_resources",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_resources(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
        /// Cursor specifying where to start for pagination
        ///
        /// This cursor cannot be derived manually client-side. Instead, you must
        /// call this endpoint once without this query parameter specified, and
        /// then use the cursor returned in the X-Aptos-Cursor header in the
        /// response.
        start: Query<Option<StateKeyWrapper>>,
        /// Max number of account resources to retrieve
        ///
        /// If not provided, defaults to default page size.
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<MoveResource>> {
        fail_point_poem("endpoint_get_account_resources")?;
        self.context
            .check_api_output_enabled("Get account resources", &accept_type)?;
        let account = Account::new(
            self.context.clone(),
            address.0,
            ledger_version.0,
            start.0.map(StateKey::from),
            limit.0,
        )?;
        account.resources(&accept_type)
    }

    /// Get account modules
    ///
    /// Retrieves all account modules' bytecode for a given account at a specific ledger version.
    /// If the ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/modules",
        method = "get",
        operation_id = "get_account_modules",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_modules(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
        /// Cursor specifying where to start for pagination
        ///
        /// This cursor cannot be derived manually client-side. Instead, you must
        /// call this endpoint once without this query parameter specified, and
        /// then use the cursor returned in the X-Aptos-Cursor header in the
        /// response.
        start: Query<Option<StateKeyWrapper>>,
        /// Max number of account modules to retrieve
        ///
        /// If not provided, defaults to default page size.
        limit: Query<Option<u16>>,
    ) -> BasicResultWith404<Vec<MoveModuleBytecode>> {
        fail_point_poem("endpoint_get_account_modules")?;
        self.context
            .check_api_output_enabled("Get account modules", &accept_type)?;
        let account = Account::new(
            self.context.clone(),
            address.0,
            ledger_version.0,
            start.0.map(StateKey::from),
            limit.0,
        )?;
        account.modules(&accept_type)
    }
}

/// A struct representing Account related lookups for resources and modules
pub struct Account {
    context: Arc<Context>,
    /// Address of account
    address: Address,
    /// Lookup ledger version
    ledger_version: u64,
    /// Where to start for pagination
    start: Option<StateKey>,
    /// Max number of items to retrieve
    limit: Option<u16>,
    /// Current ledger info
    pub latest_ledger_info: LedgerInfo,
}

impl Account {
    /// Creates a new account struct and determines the current ledger info, and determines the
    /// ledger version to query
    pub fn new(
        context: Arc<Context>,
        address: Address,
        requested_ledger_version: Option<U64>,
        start: Option<StateKey>,
        limit: Option<u16>,
    ) -> Result<Self, BasicErrorWith404> {
        // Use the latest ledger version, or the requested associated version
        let (latest_ledger_info, requested_ledger_version) = context
            .get_latest_ledger_info_and_verify_lookup_version(
                requested_ledger_version.map(|inner| inner.0),
            )?;

        Ok(Self {
            context,
            address,
            ledger_version: requested_ledger_version,
            start,
            limit,
            latest_ledger_info,
        })
    }

    // These functions map directly to endpoint functions.

    /// Retrieves the [`AccountData`] for the associated account
    ///
    /// * JSON: Return a JSON encoded version of [`AccountData`]
    /// * BCS: Return a BCS encoded version of [`AccountData`]
    pub fn account(self, accept_type: &AcceptType) -> BasicResultWith404<AccountData> {
        // Retrieve the Account resource and convert it accordingly
        let state_value = self.get_account_resource()?;

        // Convert the AccountResource into the summary object AccountData
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

    pub fn get_account_resource(&self) -> Result<Vec<u8>, BasicErrorWith404> {
        let state_key = StateKey::AccessPath(AccessPath::resource_access_path(ResourceKey::new(
            self.address.into(),
            AccountResource::struct_tag(),
        )));

        let state_value = self.context.get_state_value_poem(
            &state_key,
            self.ledger_version,
            &self.latest_ledger_info,
        )?;

        state_value.ok_or_else(|| {
            account_not_found(self.address, self.ledger_version, &self.latest_ledger_info)
        })
    }

    /// Retrieves the move resources associated with the account
    ///
    /// * JSON: Return a JSON encoded version of [`Vec<MoveResource>`]
    /// * BCS: Return a sorted BCS encoded version of BCS encoded resources [`BTreeMap<StructTag, Vec<u8>>`]
    ///
    /// Note: For the BCS response, if results are being returned in pages, i.e. with the
    /// `start` and `limit` query parameters, the results will only be sorted within each page.
    pub fn resources(self, accept_type: &AcceptType) -> BasicResultWith404<Vec<MoveResource>> {
        // check account exists
        self.get_account_resource()?;
        let max_account_resources_page_size = self.context.max_account_resources_page_size();
        let (resources, next_state_key) = self
            .context
            .get_resources_by_pagination(
                self.address.into(),
                self.start.as_ref(),
                self.ledger_version,
                // Just use the max as the default
                determine_limit(
                    self.limit,
                    max_account_resources_page_size,
                    max_account_resources_page_size,
                    &self.latest_ledger_info,
                )? as u64,
            )
            .context("Failed to get resources from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                // Resolve the BCS encoded versions into `MoveResource`s
                let move_resolver = self.context.move_resolver_poem(&self.latest_ledger_info)?;
                let converted_resources = move_resolver
                    .as_converter(self.context.db.clone())
                    .try_into_resources(resources.iter().map(|(k, v)| (k.clone(), v.as_slice())))
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
                .map(|v| v.with_cursor(next_state_key))
            }
            AcceptType::Bcs => {
                // Put resources in a BTreeMap to ensure they're ordered the same every time
                let resources: BTreeMap<StructTag, Vec<u8>> = resources.into_iter().collect();
                BasicResponse::try_from_bcs((
                    resources,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
                .map(|v| v.with_cursor(next_state_key))
            }
        }
    }

    /// Retrieves the move modules' bytecode associated with the account
    ///
    /// * JSON: Return a JSON encoded version of [`Vec<MoveModuleBytecode>`] with parsed ABIs
    /// * BCS: Return a sorted BCS encoded version of bytecode [`BTreeMap<MoveModuleId, Vec<u8>>`]
    ///
    /// Note: For the BCS response, if results are being returned in pages, i.e. with the
    /// `start` and `limit` query parameters, the results will only be sorted within each page.
    pub fn modules(self, accept_type: &AcceptType) -> BasicResultWith404<Vec<MoveModuleBytecode>> {
        // check account exists
        self.get_account_resource()?;
        let max_account_modules_page_size = self.context.max_account_modules_page_size();
        let (modules, next_state_key) = self
            .context
            .get_modules_by_pagination(
                self.address.into(),
                self.start.as_ref(),
                self.ledger_version,
                // Just use the max as the default
                determine_limit(
                    self.limit,
                    max_account_modules_page_size,
                    max_account_modules_page_size,
                    &self.latest_ledger_info,
                )? as u64,
            )
            .context("Failed to get modules from storage")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                // Read bytecode and parse ABIs for output
                let mut converted_modules = Vec::new();
                for (_, module) in modules {
                    converted_modules.push(
                        MoveModuleBytecode::new(module.clone())
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
                .map(|v| v.with_cursor(next_state_key))
            }
            AcceptType::Bcs => {
                // Sort modules by name
                let modules: BTreeMap<MoveModuleId, Vec<u8>> = modules
                    .into_iter()
                    .map(|(key, value)| (key.into(), value))
                    .collect();
                BasicResponse::try_from_bcs((
                    modules,
                    &self.latest_ledger_info,
                    BasicResponseStatus::Ok,
                ))
                .map(|v| v.with_cursor(next_state_key))
            }
        }
    }

    /// Retrieves an event key from a [`MoveStructTag`] and a [`Identifier`] field name
    ///
    /// e.g. If there's the `CoinStore` module, it has a field named `withdraw_events` for
    /// the withdraw events to lookup the key
    pub fn find_event_key(
        &self,
        struct_tag: MoveStructTag,
        field_name: Identifier,
    ) -> Result<EventKey, BasicErrorWith404> {
        // Parse the struct tag
        let struct_tag: StructTag = struct_tag
            .try_into()
            .context("Given event handle was invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &self.latest_ledger_info,
                )
            })?;

        // Find the resource and retrieve the struct field
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

        // Deserialize the event handle to retrieve the key
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

    /// Find a resource associated with an account
    fn find_resource(
        &self,
        struct_tag: &StructTag,
    ) -> Result<Vec<(Identifier, MoveValue)>, BasicErrorWith404> {
        let state_key = StateKey::AccessPath(AccessPath::resource_access_path(ResourceKey::new(
            self.address.into(),
            struct_tag.clone(),
        )));
        let state_value_bytes = self
            .context
            .db
            .get_state_value_by_version(&state_key, self.ledger_version)
            .context("Failed to read state value from db")
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &self.latest_ledger_info,
                )
            })?
            .ok_or_else(|| {
                resource_not_found(
                    self.address,
                    struct_tag,
                    self.ledger_version,
                    &self.latest_ledger_info,
                )
            })?
            .into_bytes();

        // Convert to fields in move struct
        let move_resolver = self.context.move_resolver_poem(&self.latest_ledger_info)?;
        move_resolver
            .as_converter(self.context.db.clone())
            .move_struct_fields(struct_tag, state_value_bytes.as_slice())
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
