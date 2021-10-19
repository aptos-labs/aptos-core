// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::Context,
    param::{AddressParam, LedgerVersionParam},
};

use diem_api_types::{Address, Error, LedgerInfo, MoveModuleBytecode, Response, TransactionId};
use diem_types::account_state::AccountState;

use anyhow::Result;
use warp::{Filter, Rejection, Reply};

pub fn routes(context: Context) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    get_account_resources(context.clone())
        .or(get_account_resources_by_ledger_version(context.clone()))
        .or(get_account_modules(context.clone()))
        .or(get_account_modules_by_ledger_version(context))
}

// GET /accounts/<address>/resources
pub fn get_account_resources(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / AddressParam / "resources")
        .and(warp::get())
        .and(context.filter())
        .map(|address, ctx| (None, address, ctx))
        .untuple_one()
        .and_then(handle_get_account_resources)
}

// GET /ledger/<version>/accounts/<address>/resources
pub fn get_account_resources_by_ledger_version(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("ledger" / LedgerVersionParam / "accounts" / AddressParam / "resources")
        .and(warp::get())
        .and(context.filter())
        .map(|version, address, ctx| (Some(version), address, ctx))
        .untuple_one()
        .and_then(handle_get_account_resources)
}

// GET /accounts/<address>/modules
pub fn get_account_modules(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("accounts" / AddressParam / "modules")
        .and(warp::get())
        .and(context.filter())
        .map(|address, ctx| (None, address, ctx))
        .untuple_one()
        .and_then(handle_get_account_modules)
}

// GET /ledger/<version>/accounts/<address>/modules
pub fn get_account_modules_by_ledger_version(
    context: Context,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("ledger" / LedgerVersionParam / "accounts" / AddressParam / "modules")
        .and(warp::get())
        .and(context.filter())
        .map(|version, address, ctx| (Some(version), address, ctx))
        .untuple_one()
        .and_then(handle_get_account_modules)
}

async fn handle_get_account_resources(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(ledger_version, address, context)?.resources()?)
}

async fn handle_get_account_modules(
    ledger_version: Option<LedgerVersionParam>,
    address: AddressParam,
    context: Context,
) -> Result<impl Reply, Rejection> {
    Ok(Account::new(ledger_version, address, context)?.modules()?)
}

struct Account {
    ledger_version: u64,
    address: Address,
    latest_ledger_info: LedgerInfo,
    context: Context,
}

impl Account {
    pub fn new(
        ledger_version: Option<LedgerVersionParam>,
        address: AddressParam,
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

        Ok(Self {
            ledger_version,
            address: address.parse("account address")?,
            latest_ledger_info,
            context,
        })
    }

    pub fn resources(self) -> Result<impl Reply, Error> {
        let resources = self
            .context
            .move_converter()
            .try_into_resources(self.account_state()?.get_resources())?;
        Response::new(self.latest_ledger_info, &resources)
    }

    pub fn modules(self) -> Result<impl Reply, Error> {
        let modules = self
            .account_state()?
            .into_modules()
            .map(MoveModuleBytecode::new)
            .map(|m| m.ensure_abi())
            .collect::<Result<Vec<MoveModuleBytecode>>>()?;
        Response::new(self.latest_ledger_info, &modules)
    }

    fn account_state(&self) -> Result<AccountState, Error> {
        let state = self
            .context
            .get_account_state(self.address.into(), self.ledger_version)?
            .ok_or_else(|| self.account_not_found())?;
        Ok(state)
    }

    fn account_not_found(&self) -> Error {
        Error::not_found(
            "account",
            format!(
                "address({}) and ledger version({})",
                self.address, self.ledger_version,
            ),
            self.latest_ledger_info.version(),
        )
    }
}
