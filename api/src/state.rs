// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{context::Context, param::LedgerVersionParam};
use aptos_api_types::{
    AsConverter, Error, LedgerInfo, MoveModuleBytecode, Response, TransactionId,
};
use aptos_state_view::StateView;
use aptos_types::{access_path::AccessPath, state_store::state_key::StateKey};
use aptos_vm::data_cache::AsMoveResolver;
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, ResourceKey, StructTag},
};
use storage_interface::state_view::DbStateView;
use warp::{http::StatusCode, Reply};

pub(crate) struct State {
    state_view: DbStateView,
    ledger_version: aptos_types::transaction::Version,
    latest_ledger_info: LedgerInfo,
}

impl State {
    pub fn new(
        ledger_version: Option<LedgerVersionParam>,
        context: Context,
    ) -> Result<Self, Error> {
        let latest_ledger_info = context.get_latest_ledger_info()?;
        let ledger_version = ledger_version
            .map(|v| v.parse("ledger version"))
            .unwrap_or_else(|| Ok(latest_ledger_info.version()))?;

        if ledger_version > latest_ledger_info.version() {
            return Err(Error::not_found(
                "ledger",
                TransactionId::Version(ledger_version),
                latest_ledger_info.version(),
            ));
        }

        let state_view = context.state_view_at_version(ledger_version)?;

        Ok(Self {
            state_view,
            ledger_version,
            latest_ledger_info,
        })
    }

    pub fn resource(
        self,
        address: AccountAddress,
        struct_tag: StructTag,
    ) -> Result<impl Reply, Error> {
        let resource_key = ResourceKey::new(address, struct_tag.clone());
        let access_path = AccessPath::resource_access_path(resource_key.clone());
        let state_key = StateKey::AccessPath(access_path);
        let bytes = self
            .state_view
            .get_state_value(&state_key)?
            .ok_or_else(|| Error::not_found("Resource", resource_key, self.ledger_version))?;

        let resource = self
            .state_view
            .as_move_resolver()
            .as_converter()
            .try_into_resource(&struct_tag, &bytes)?;
        Response::new(self.latest_ledger_info, &resource)
    }

    pub fn module(self, address: AccountAddress, name: Identifier) -> Result<impl Reply, Error> {
        let module_id = ModuleId::new(address, name);
        let access_path = AccessPath::code_access_path(module_id.clone());
        let state_key = StateKey::AccessPath(access_path);
        let bytes = self
            .state_view
            .get_state_value(&state_key)?
            .ok_or_else(|| Error::not_found("Module", module_id, self.ledger_version))?;

        let module = MoveModuleBytecode::new(bytes)
            .try_parse_abi()
            .map_err(|e| Error::from_anyhow_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        Response::new(self.latest_ledger_info, &module)
    }
}
