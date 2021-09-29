// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::{Error, LedgerInfo};
use diem_types::{
    account_address::AccountAddress, account_state_blob::AccountStateBlob, chain_id::ChainId,
    ledger_info::LedgerInfoWithSignatures, protocol_spec::DpnProto,
    transaction::default_protocol::TransactionListWithProof,
};
use storage_interface::MoveDbReader;

use anyhow::Result;
use std::{borrow::Borrow, convert::Infallible, sync::Arc};
use warp::Filter;

// Context holds application scope context
#[derive(Clone)]
pub struct Context {
    chain_id: ChainId,
    db: Arc<dyn MoveDbReader<DpnProto>>,
}

impl Context {
    pub fn new(chain_id: ChainId, db: Arc<dyn MoveDbReader<DpnProto>>) -> Self {
        Self { chain_id, db }
    }

    pub fn db(&self) -> &dyn MoveDbReader<DpnProto> {
        self.db.borrow()
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn filter(self) -> impl Filter<Extract = (Context,), Error = Infallible> + Clone {
        warp::any().map(move || self.clone())
    }

    pub fn get_latest_ledger_info(&self) -> Result<LedgerInfo, Error> {
        Ok(LedgerInfo::new(
            &self.chain_id(),
            &self.get_latest_ledger_info_with_signatures()?,
        ))
    }

    pub fn get_latest_ledger_info_with_signatures(&self) -> Result<LedgerInfoWithSignatures> {
        self.db.get_latest_ledger_info()
    }

    pub fn get_account_state_blob(
        &self,
        account: AccountAddress,
        version: u64,
    ) -> Result<Option<AccountStateBlob>> {
        let (account_state_blob, _) = self
            .db
            .get_account_state_with_proof_by_version(account, version)?;
        Ok(account_state_blob)
    }

    pub fn get_transactions(
        &self,
        start_version: u64,
        limit: u16,
        ledger_version: u64,
    ) -> Result<TransactionListWithProof> {
        self.db
            .get_transactions(start_version, limit as u64, ledger_version, true)
    }
}
