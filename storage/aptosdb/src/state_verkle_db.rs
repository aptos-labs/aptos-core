// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use schemadb::DB;
use std::sync::Arc;

#[derive(Debug)]
pub struct StateVerkleDb {}

impl StateVerkleDb {
    pub(crate) fn new(state_verkle_rocksdb: Arc<DB>) -> Self {
        todo!()
    }
}
