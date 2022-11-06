// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::node_type::{Node, NodeKey};
use anyhow::Result;
use aptos_crypto::hash::CryptoHash;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::state_value::StateValue;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;

pub mod node_type;
pub mod restore;

pub trait TreeWriter<K>: Send + Sync {
    fn write_node_batch(&self, node_batch: &HashMap<NodeKey, Node<K>>) -> Result<()>;
}

pub trait TreeReader<K> {}

pub trait Key: Clone + Serialize + DeserializeOwned + Send + Sync + 'static {
    fn key_size(&self) -> usize;
}

impl Key for StateKey {
    fn key_size(&self) -> usize {
        self.size()
    }
}

pub trait Value: Clone + CryptoHash + Serialize + DeserializeOwned + Send + Sync {
    fn value_size(&self) -> usize;
}

impl Value for StateValue {
    fn value_size(&self) -> usize {
        self.size()
    }
}
