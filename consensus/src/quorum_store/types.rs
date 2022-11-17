// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::SignedTransaction;
use bcs::to_bytes;
use serde::{Deserialize, Serialize};
use std::mem;

pub(crate) type BatchId = u64;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SerializedTransaction {
    bytes: Vec<u8>,
}

#[allow(dead_code)]
impl SerializedTransaction {
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn from_signed_txn(txn: &SignedTransaction) -> Self {
        Self {
            bytes: to_bytes(&txn).unwrap(),
        }
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn bytes(&self) -> &Vec<u8> {
        &self.bytes
    }

    pub fn take_bytes(&mut self) -> Vec<u8> {
        mem::take(&mut self.bytes)
    }
}
