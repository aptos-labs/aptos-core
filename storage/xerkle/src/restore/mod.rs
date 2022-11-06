// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{TreeReader, TreeWriter};
use anyhow::{ensure, Result};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::HashValue;
use aptos_types::transaction::Version;
use std::sync::Arc;

pub struct XerkleRestore<K> {
    store: Arc<dyn TreeWriter<K>>,
}

impl<K> XerkleRestore<K> {
    pub fn new_overwrite<D: 'static + TreeWriter<K>>(
        store: Arc<D>,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<Self> {
        todo!()
    }
}

impl<K> XerkleRestore<K>
where
    K: crate::Key + CryptoHash + 'static,
{
    pub fn new<D: 'static + TreeReader<K> + TreeWriter<K>>(
        store: Arc<D>,
        version: Version,
        expected_root_hash: HashValue,
        async_commit: bool,
    ) -> Result<Self> {
        todo!()
    }
}
