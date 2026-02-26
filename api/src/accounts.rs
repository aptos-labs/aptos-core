// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    accept_type::AcceptType,
    context::Context,
    page::determine_limit,
    response::{account_not_found, resource_not_found, struct_field_not_found},
    response_axum::{AptosErrorResponse, AptosResponse},
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    AccountData, Address, AptosErrorCode, AsConverter, AssetType, LedgerInfo, MoveModuleBytecode,
    MoveModuleId, MoveResource, MoveStructTag, U64,
};
use aptos_sdk::types::{get_paired_fa_metadata_address, get_paired_fa_primary_store_address};
use aptos_types::{
    account_config::{
        AccountResource, CoinStoreResourceUntyped, ConcurrentFungibleBalanceResource,
        FungibleStoreResource, ObjectGroupResource,
    },
    event::{EventHandle, EventKey},
    state_store::state_key::StateKey,
};
use move_core_types::{
    identifier::Identifier, language_storage::StructTag, move_resource::MoveStructType,
};
use std::{collections::BTreeMap, convert::TryInto, str::FromStr, sync::Arc};

/// A struct representing Account related lookups for resources and modules
pub struct Account {
    context: Arc<Context>,
    /// Address of account
    address: Address,
    /// Lookup ledger version
    pub ledger_version: u64,
    /// Where to start for pagination
    start: Option<StateKey>,
    /// Max number of items to retrieve
    limit: Option<u16>,
    /// Current ledger info
    pub latest_ledger_info: LedgerInfo,
}

impl Account {
    pub fn new(
        context: Arc<Context>,
        address: Address,
        requested_ledger_version: Option<U64>,
        start: Option<StateKey>,
        limit: Option<u16>,
    ) -> Result<Self, AptosErrorResponse> {
        let (latest_ledger_info, requested_version) = context
            .get_latest_ledger_info_and_verify_lookup_version::<AptosErrorResponse>(
                requested_ledger_version.map(|inner| inner.0),
            )?;

        Ok(Self {
            context,
            address,
            ledger_version: requested_version,
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
    pub fn account(
        self,
        accept_type: &AcceptType,
    ) -> Result<AptosResponse<AccountData>, AptosErrorResponse> {
        let state_value_opt = self.get_account_resource()?;

        let account_resource = if let Some(state_value) = &state_value_opt {
            let account_resource: AccountResource = bcs::from_bytes(state_value)
                .context("Internal error deserializing response from DB")
                .map_err(|err| {
                    AptosErrorResponse::internal(
                        err,
                        AptosErrorCode::InternalError,
                        Some(&self.latest_ledger_info),
                    )
                })?;
            account_resource
        } else {
            let stateless_account_enabled = self
                .context
                .feature_enabled(
                    aptos_types::on_chain_config::FeatureFlag::DEFAULT_ACCOUNT_RESOURCE,
                )
                .context("Failed to check if stateless account is enabled")
                .map_err(|_| {
                    AptosErrorResponse::internal(
                        "Failed to check if stateless account is enabled",
                        AptosErrorCode::InternalError,
                        Some(&self.latest_ledger_info),
                    )
                })?;
            if stateless_account_enabled {
                AccountResource::new_stateless(*self.address.inner())
            } else {
                Err(account_not_found::<AptosErrorResponse>(
                    self.address,
                    self.ledger_version,
                    &self.latest_ledger_info,
                ))?
            }
        };

        match accept_type {
            AcceptType::Json => {
                AptosResponse::try_from_json(account_resource.into(), &self.latest_ledger_info)
            },
            AcceptType::Bcs => AptosResponse::try_from_encoded(
                state_value_opt.unwrap_or_else(|| bcs::to_bytes(&account_resource).unwrap()),
                &self.latest_ledger_info,
            ),
        }
    }

    pub fn balance(
        &self,
        asset_type: AssetType,
        accept_type: &AcceptType,
    ) -> Result<AptosResponse<u64>, AptosErrorResponse> {
        let (fa_metadata_address, mut balance) = match asset_type {
            AssetType::Coin(move_struct_tag) => {
                let coin_store_type_tag =
                    StructTag::from_str(&format!("0x1::coin::CoinStore<{}>", move_struct_tag))
                        .map_err(|err| {
                            AptosErrorResponse::internal(
                                err,
                                AptosErrorCode::InternalError,
                                Some(&self.latest_ledger_info),
                            )
                        })?;
                let state_value = self.context.get_state_value_poem::<AptosErrorResponse>(
                    &StateKey::resource(&self.address.into(), &coin_store_type_tag).map_err(
                        |err| {
                            AptosErrorResponse::internal(
                                err,
                                AptosErrorCode::InternalError,
                                Some(&self.latest_ledger_info),
                            )
                        },
                    )?,
                    self.ledger_version,
                    &self.latest_ledger_info,
                )?;
                let coin_balance = match state_value {
                    None => 0,
                    Some(bytes) => bcs::from_bytes::<CoinStoreResourceUntyped>(&bytes)
                        .map_err(|err| {
                            AptosErrorResponse::internal(
                                err,
                                AptosErrorCode::InternalError,
                                Some(&self.latest_ledger_info),
                            )
                        })?
                        .coin(),
                };
                (
                    get_paired_fa_metadata_address(&move_struct_tag),
                    coin_balance,
                )
            },
            AssetType::FungibleAsset(fa_metadata_adddress) => (fa_metadata_adddress.into(), 0),
        };
        let primary_fungible_store_address =
            get_paired_fa_primary_store_address(self.address.into(), fa_metadata_address);
        if let Some(data_blob) = self.context.get_state_value_poem::<AptosErrorResponse>(
            &StateKey::resource_group(
                &primary_fungible_store_address,
                &ObjectGroupResource::struct_tag(),
            ),
            self.ledger_version,
            &self.latest_ledger_info,
        )? {
            if let Ok(object_group) = bcs::from_bytes::<ObjectGroupResource>(&data_blob) {
                if let Some(fa_store) = object_group.group.get(&FungibleStoreResource::struct_tag())
                {
                    let fa_store_resource = bcs::from_bytes::<FungibleStoreResource>(fa_store)
                        .map_err(|err| {
                            AptosErrorResponse::internal(
                                err,
                                AptosErrorCode::InternalError,
                                Some(&self.latest_ledger_info),
                            )
                        })?;
                    if fa_store_resource.balance != 0 {
                        balance += fa_store_resource.balance();
                    } else if let Some(concurrent_fa_balance) = object_group
                        .group
                        .get(&ConcurrentFungibleBalanceResource::struct_tag())
                    {
                        let concurrent_fa_balance_resource =
                            bcs::from_bytes::<ConcurrentFungibleBalanceResource>(
                                concurrent_fa_balance,
                            )
                            .map_err(|err| {
                                AptosErrorResponse::internal(
                                    err,
                                    AptosErrorCode::InternalError,
                                    Some(&self.latest_ledger_info),
                                )
                            })?;
                        balance += concurrent_fa_balance_resource.balance();
                    }
                }
            }
        }
        match accept_type {
            AcceptType::Json => AptosResponse::try_from_json(balance, &self.latest_ledger_info),
            AcceptType::Bcs => AptosResponse::try_from_encoded(
                bcs::to_bytes(&balance).unwrap(),
                &self.latest_ledger_info,
            ),
        }
    }

    /// Retrieves the account resource for the associated account
    fn get_account_resource(&self) -> Result<Option<Vec<u8>>, AptosErrorResponse> {
        let state_key =
            StateKey::resource_typed::<AccountResource>(self.address.inner()).map_err(|e| {
                AptosErrorResponse::internal(
                    e,
                    AptosErrorCode::InternalError,
                    Some(&self.latest_ledger_info),
                )
            })?;

        self.context.get_state_value_poem::<AptosErrorResponse>(
            &state_key,
            self.ledger_version,
            &self.latest_ledger_info,
        )
    }

    /// Retrieves the move resources associated with the account
    ///
    /// * JSON: Return a JSON encoded version of [`Vec<MoveResource>`]
    /// * BCS: Return a sorted BCS encoded version of BCS encoded resources [`BTreeMap<StructTag, Vec<u8>>`]
    ///
    /// Note: For the BCS response, if results are being returned in pages, i.e. with the
    /// `start` and `limit` query parameters, the results will only be sorted within each page.
    pub fn resources(
        self,
        accept_type: &AcceptType,
    ) -> Result<AptosResponse<Vec<MoveResource>>, AptosErrorResponse> {
        let max_account_resources_page_size = self.context.max_account_resources_page_size();
        let (resources, next_state_key) = self
            .context
            .get_resources_by_pagination(
                self.address.into(),
                self.start.as_ref(),
                self.ledger_version,
                determine_limit::<AptosErrorResponse>(
                    self.limit,
                    max_account_resources_page_size,
                    max_account_resources_page_size,
                    &self.latest_ledger_info,
                )? as u64,
            )
            .context("Failed to get resources from storage")
            .map_err(|err| {
                AptosErrorResponse::internal(
                    err,
                    AptosErrorCode::InternalError,
                    Some(&self.latest_ledger_info),
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                let state_view = self
                    .context
                    .latest_state_view_poem::<AptosErrorResponse>(&self.latest_ledger_info)?;
                let converter = state_view
                    .as_converter(self.context.db.clone(), self.context.indexer_reader.clone());
                let converted_resources = converter
                    .try_into_resources(resources.iter().map(|(k, v)| (k.clone(), v.as_slice())))
                    .context("Failed to build move resource response from data in DB")
                    .map_err(|err| {
                        AptosErrorResponse::internal(
                            err,
                            AptosErrorCode::InternalError,
                            Some(&self.latest_ledger_info),
                        )
                    })?;
                AptosResponse::try_from_json(converted_resources, &self.latest_ledger_info)
                    .map(|v| v.with_cursor(next_state_key))
            },
            AcceptType::Bcs => {
                let resources: BTreeMap<StructTag, Vec<u8>> = resources.into_iter().collect();
                AptosResponse::try_from_bcs(resources, &self.latest_ledger_info)
                    .map(|v| v.with_cursor(next_state_key))
            },
        }
    }

    /// Retrieves the move modules' bytecode associated with the account
    ///
    /// * JSON: Return a JSON encoded version of [`Vec<MoveModuleBytecode>`] with parsed ABIs
    /// * BCS: Return a sorted BCS encoded version of bytecode [`BTreeMap<MoveModuleId, Vec<u8>>`]
    ///
    /// Note: For the BCS response, if results are being returned in pages, i.e. with the
    /// `start` and `limit` query parameters, the results will only be sorted within each page.
    pub fn modules(
        self,
        accept_type: &AcceptType,
    ) -> Result<AptosResponse<Vec<MoveModuleBytecode>>, AptosErrorResponse> {
        let max_account_modules_page_size = self.context.max_account_modules_page_size();
        let (modules, next_state_key) = self
            .context
            .get_modules_by_pagination(
                self.address.into(),
                self.start.as_ref(),
                self.ledger_version,
                determine_limit::<AptosErrorResponse>(
                    self.limit,
                    max_account_modules_page_size,
                    max_account_modules_page_size,
                    &self.latest_ledger_info,
                )? as u64,
            )
            .context("Failed to get modules from storage")
            .map_err(|err| {
                AptosErrorResponse::internal(
                    err,
                    AptosErrorCode::InternalError,
                    Some(&self.latest_ledger_info),
                )
            })?;

        match accept_type {
            AcceptType::Json => {
                let mut converted_modules = Vec::new();
                for (_, module) in modules {
                    converted_modules.push(
                        MoveModuleBytecode::new(module.clone())
                            .try_parse_abi()
                            .context("Failed to parse move module ABI")
                            .map_err(|err| {
                                AptosErrorResponse::internal(
                                    err,
                                    AptosErrorCode::InternalError,
                                    Some(&self.latest_ledger_info),
                                )
                            })?,
                    );
                }
                AptosResponse::try_from_json(converted_modules, &self.latest_ledger_info)
                    .map(|v| v.with_cursor(next_state_key))
            },
            AcceptType::Bcs => {
                let modules: BTreeMap<MoveModuleId, Vec<u8>> = modules
                    .into_iter()
                    .map(|(key, value)| (key.into(), value))
                    .collect();
                AptosResponse::try_from_bcs(modules, &self.latest_ledger_info)
                    .map(|v| v.with_cursor(next_state_key))
            },
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
    ) -> Result<EventKey, AptosErrorResponse> {
        let struct_tag: StructTag = (&struct_tag)
            .try_into()
            .context("Given event handle was invalid")
            .map_err(|err| {
                AptosErrorResponse::bad_request(
                    err,
                    AptosErrorCode::InvalidInput,
                    Some(&self.latest_ledger_info),
                )
            })?;

        let (_, resource) = self.find_resource(&struct_tag)?;
        let (_id, value) = resource
            .into_iter()
            .find(|(id, _)| id == &field_name)
            .ok_or_else(|| {
                struct_field_not_found::<AptosErrorResponse>(
                    self.address,
                    &struct_tag,
                    &field_name,
                    self.ledger_version,
                    &self.latest_ledger_info,
                )
            })?;

        let event_handle_bytes = bcs::to_bytes(&value)
            .context("Failed to serialize event handle from storage")
            .map_err(|err| {
                AptosErrorResponse::internal(
                    err,
                    AptosErrorCode::InternalError,
                    Some(&self.latest_ledger_info),
                )
            })?;
        // Deserialization may fail because the bytes are not EventHandle struct type.
        let event_handle: EventHandle = bcs::from_bytes(&event_handle_bytes)
            .context(format!(
                "Deserialization error, field({}) type is not a EventHandle struct",
                field_name
            ))
            .map_err(|err| {
                AptosErrorResponse::bad_request(
                    err,
                    AptosErrorCode::InvalidInput,
                    Some(&self.latest_ledger_info),
                )
            })?;
        Ok(*event_handle.key())
    }

    /// Find a resource associated with an account. If the resource is an enum variant,
    /// returns the variant name in the option.
    fn find_resource(
        &self,
        resource_type: &StructTag,
    ) -> Result<
        (
            Option<Identifier>,
            Vec<(Identifier, move_core_types::value::MoveValue)>,
        ),
        AptosErrorResponse,
    > {
        let (ledger_info, requested_ledger_version, state_view) =
            self.context
                .state_view::<AptosErrorResponse>(Some(self.ledger_version))?;

        let bytes = state_view
            .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
            .find_resource(&state_view, self.address, resource_type)
            .context(format!(
                "Failed to query DB to check for {} at {}",
                resource_type.to_canonical_string(),
                self.address
            ))
            .map_err(|err| {
                AptosErrorResponse::internal(
                    err,
                    AptosErrorCode::InternalError,
                    Some(&ledger_info),
                )
            })?
            .ok_or_else(|| {
                resource_not_found::<AptosErrorResponse>(
                    self.address,
                    resource_type,
                    requested_ledger_version,
                    &ledger_info,
                )
            })?;

        state_view
            .as_converter(self.context.db.clone(), self.context.indexer_reader.clone())
            .move_struct_fields(resource_type, &bytes)
            .context("Failed to convert move structs from storage")
            .map_err(|err| {
                AptosErrorResponse::internal(
                    err,
                    AptosErrorCode::InternalError,
                    Some(&ledger_info),
                )
            })
    }
}

