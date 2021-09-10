// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_types::{chain_id::ChainId, ledger_info::LedgerInfoWithSignatures};
use storage_interface::MoveDbReader;

use anyhow::Result;
use std::{borrow::Borrow, sync::Arc};

// Context holds application scope context
pub struct Context {
    chain_id: ChainId,
    db: Arc<dyn MoveDbReader>,
}

impl Context {
    pub fn new(chain_id: ChainId, db: Arc<dyn MoveDbReader>) -> Self {
        Self { chain_id, db }
    }

    pub fn db(&self) -> &dyn MoveDbReader {
        self.db.borrow()
    }

    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    pub fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        self.db.get_latest_ledger_info()
    }
}
