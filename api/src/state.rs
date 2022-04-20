// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::Context,
    failpoint::fail_point,
    metrics::metrics,
    param::{AddressParam, LedgerVersionParam, MoveIdentifierParam, MoveStructTagParam},
    version::Version,
};
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
use std::convert::TryInto;
use storage_interface::state_view::DbStateView;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

// GET /accounts/<address>/resource/<resource_type>
pub fn get_account_resource(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("accounts" / AddressParam / "resource" / MoveStructTagParam)
        .and(warp::get())
        .and(context.filter())
        .and(warp::query::<Version>())
        .map(|address, struct_tag, ctx, version: Version| {
            (version.version, address, struct_tag, ctx)
        })
        .untuple_one()
        .and_then(handle_get_account_resource)
        .with(metrics("get_account_resource"))
        .boxed()
}

// GET /state/module/<address>/<module_name>
pub fn get_account_module(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("accounts" / AddressParam / "module" / MoveIdentifierParam)
        .and(warp::get())
        .and(context.filter())
        .and(warp::query::<Version>())
        .map(|address, name, ctx, version: Version| (version.version, address, name, ctx))
        .untuple_one()
        .and_then(handle_get_account_module)
        .with(metrics("get_account_module"))
        .boxed()
}

async fn handle_get_account_resource(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    struct_tag: MoveStructTagParam,
    context: Context,
) -> anyhow::Result<impl Reply, Rejection> {
    fail_point("endpoint_query_resource")?;
    let struct_tag = struct_tag.parse("struct tag")?;
    Ok(State::new(ledger_version, context)?.resource(
        address.parse("account address")?.into(),
        struct_tag
            .clone()
            .try_into()
            .map_err(|_| Error::invalid_param("resource_type", struct_tag))?,
    )?)
}

async fn handle_get_account_module(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    name: MoveIdentifierParam,
    context: Context,
) -> anyhow::Result<impl Reply, Rejection> {
    fail_point("endpoint_get_account_module")?;
    Ok(State::new(ledger_version, context)?.module(
        address.parse("account address")?.into(),
        name.parse("module name")?,
    )?)
}

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
            .map_err(Error::internal)?;
        Response::new(self.latest_ledger_info, &module)
    }
}
