// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::convert::TryFrom;
use std::sync::Arc;

use super::accept_type::AcceptType;
use super::response::deserialize_from_bcs;
use super::AptosErrorResponse;
use super::{response::AptosResult, ApiTags, AptosResponse};
use crate::context::Context;
use crate::failpoint::fail_point_poem;
use aptos_api_types::LedgerInfo;
use aptos_api_types::{AccountData, Address, TransactionId};
use aptos_types::access_path::AccessPath;
use aptos_types::account_config::AccountResource;
use aptos_types::state_store::state_key::StateKey;
use move_deps::move_core_types::{
    language_storage::{ResourceKey, StructTag},
    move_resource::MoveStructType,
};
use poem::web::Accept;
use poem_openapi::param::Query;
use poem_openapi::{param::Path, OpenApi};

pub struct AccountsApi {
    pub context: Arc<Context>,
}

#[OpenApi]
impl AccountsApi {
    /// get_account
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

    fn resource_not_found(&self, struct_tag: &StructTag) -> AptosErrorResponse {
        AptosErrorResponse::not_found(
            "resource",
            format!(
                "address({}), struct tag({}) and ledger version({})",
                self.address, struct_tag, self.ledger_version,
            ),
            self.latest_ledger_info.version(),
        )
    }
}

// TODO: For the BCS response type, instead of constructing the Rust type from
// BCS just to serialize it back again, return the bytes directly. This is an
// example of doing that, but it requires extensive testing:
/*
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
