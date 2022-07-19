// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;
use std::sync::Arc;

use super::accept_type::AcceptType;
use super::response::deserialize_from_bcs;
use super::{response::AptosResult, ApiTags, AptosResponse};
use super::{AptosError, AptosErrorCode, AptosErrorResponse};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use anyhow::format_err;
use aptos_api_types::{AccountData, Address, AsConverter, TransactionId};
use aptos_api_types::{LedgerInfo, MoveResource};
use aptos_types::access_path::AccessPath;
use aptos_types::account_config::AccountResource;
use aptos_types::account_state::AccountState;
use aptos_types::state_store::state_key::StateKey;
use move_deps::move_core_types::{
    language_storage::{ResourceKey, StructTag},
    move_resource::MoveStructType,
};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::payload::Json;
use poem_openapi::{param::Path, OpenApi};

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
        tag = "ApiTags::General"
    )]
    async fn get_account(
        &self,
        accept: Accept,
        address: Path<Address>,
        ledger_version: Query<Option<u64>>,
    ) -> AptosResult<AccountData> {
        fail_point_poem("endpoint_get_account")?;
        let accept_type = AcceptType::try_from(&accept)?;
        let account = Account::new(self.context.clone(), address.0, ledger_version.0)?;
        account.account(&accept_type)
    }

    /// Get account resources
    ///
    /// This API returns account resources for a specific ledger version (AKA transaction version).
    /// If not present, the latest version is used. <---- TODO Update this comment
    /// The Aptos nodes prune account state history, via a configurable time window (link).
    /// If the requested data has been pruned, the server responds with a 404
    #[oai(
        path = "/accounts/:address/resources",
        method = "get",
        operation_id = "get_account_resources",
        tag = "ApiTags::General"
    )]
    async fn get_account_resources(
        &self,
        accept: Accept,
        address: Path<Address>,
        ledger_version: Query<Option<u64>>,
    ) -> AptosResult<Vec<MoveResource>> {
        fail_point_poem("endpoint_get_account_resources")?;
        let accept_type = AcceptType::try_from(&accept)?;
        let account = Account::new(self.context.clone(), address.0, ledger_version.0)?;
        account.resources(&accept_type)
    }
}

struct Account {
    context: Arc<Context>,
    address: Address,
    ledger_version: u64,
    latest_ledger_info: LedgerInfo,
}

impl Account {
    pub fn new(
        context: Arc<Context>,
        address: Address,
        requested_ledger_version: Option<u64>,
    ) -> Result<Self, AptosErrorResponse> {
        let latest_ledger_info = context.get_latest_ledger_info_poem()?;
        let ledger_version: u64 =
            requested_ledger_version.unwrap_or_else(|| latest_ledger_info.version());

        if ledger_version > latest_ledger_info.version() {
            return Err(AptosErrorResponse::not_found(
                "ledger",
                TransactionId::Version(ledger_version),
                latest_ledger_info.version(),
            ));
        }

        Ok(Self {
            context,
            address,
            ledger_version,
            latest_ledger_info,
        })
    }

    // These functions map directly to endpoint functions.

    pub fn account(self, accept_type: &AcceptType) -> AptosResult<AccountData> {
        let state_key = StateKey::AccessPath(AccessPath::resource_access_path(ResourceKey::new(
            self.address.into(),
            AccountResource::struct_tag(),
        )));

        let state_value = self
            .context
            .get_state_value_poem(&state_key, self.ledger_version)?;

        let state_value = match state_value {
            Some(state_value) => state_value,
            None => return Err(self.resource_not_found(&AccountResource::struct_tag())),
        };

        let account_resource = deserialize_from_bcs::<AccountResource>(&state_value)?;
        let account_data: AccountData = account_resource.into();

        AptosResponse::try_from_rust_value(account_data, &self.latest_ledger_info, accept_type)
    }

    pub fn resources(self, accept_type: &AcceptType) -> AptosResult<Vec<MoveResource>> {
        let account_state = self.account_state()?;
        let resources = account_state.get_resources();
        let move_resolver = self.context.move_resolver().map_err(|e| {
            AptosErrorResponse::InternalServerError(Json(
                AptosError::new(
                    format_err!("Failed to read latest state checkpoint from DB: {}", e)
                        .to_string(),
                )
                .error_code(AptosErrorCode::ReadFromStorageError),
            ))
        })?;
        let converted_resources = move_resolver
            .as_converter()
            .try_into_resources(resources)
            .map_err(|e| {
                AptosErrorResponse::InternalServerError(Json(
                    AptosError::new(
                        format_err!(
                            "Failed to build move resource response from data in DB: {}",
                            e
                        )
                        .to_string(),
                    )
                    .error_code(AptosErrorCode::InvalidBcsInStorageError),
                ))
            })?;
        AptosResponse::<Vec<MoveResource>>::try_from_rust_value(
            converted_resources,
            &self.latest_ledger_info,
            accept_type,
        )
    }

    // Helpers for processing account state.

    fn account_state(&self) -> Result<AccountState, AptosErrorResponse> {
        let state = self
            .context
            .get_account_state(self.address.into(), self.ledger_version)
            .map_err(|e| {
                AptosErrorResponse::InternalServerError(Json(
                    AptosError::new(
                        format_err!("Failed to read account state from DB: {}", e).to_string(),
                    )
                    .error_code(AptosErrorCode::ReadFromStorageError),
                ))
            })?
            .ok_or_else(|| self.account_not_found())?;
        Ok(state)
    }

    // Helpers for building errors.

    fn account_not_found(&self) -> AptosErrorResponse {
        AptosErrorResponse::not_found(
            "account",
            format!(
                "address({}) and ledger version({})",
                self.address, self.ledger_version
            ),
            self.latest_ledger_info.version(),
        )
    }

    fn resource_not_found(&self, struct_tag: &StructTag) -> AptosErrorResponse {
        AptosErrorResponse::not_found(
            "resource",
            format!(
                "address({}), struct tag({}) and ledger version({})",
                self.address, struct_tag, self.ledger_version
            ),
            self.latest_ledger_info.version(),
        )
    }
}

// TODO: For the BCS response type, instead of constructing the Rust type from
// BCS just to serialize it back again, return the bytes directly. This is an
// example of doing that, but it requires extensive testing:
/*
        let state_values = self
            .context
            .get_state_values(self.address.into(), self.ledger_version)
            .ok_or_else(|| self.account_not_found())?;
        match accept_type {
            AcceptType::Bcs => Ok(AptosResponse::from_bcs(
                state_value,
                &self.latest_ledger_info,
            )),
            AcceptType::Json => {
                let account_resource = deserialize_from_bcs::<AccountResource>(&state_value)?;
                let account_data: AccountData = account_resource.into();
                Ok(AptosResponse::from_json(
                    account_data,
                    &self.latest_ledger_info,
                ))
            }
        }
*/
