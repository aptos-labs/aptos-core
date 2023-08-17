// Copyright © Aptos Foundation

use primitive_types::{H256, U256};
use serde::{Deserialize, Serialize};
use aptos_table_natives::{TableHandle, TableResolver};
use crate::eth_address::EthAddress;
use crate::utils::{read_h256_from_bytes, read_u256_from_move_bytes};

#[derive(Clone)]
pub struct IO<'a> {
    pub(crate) resolver: &'a dyn TableResolver,
    pub(crate) nonce_table_handle: TableHandle,
    pub(crate) balance_table_handle: TableHandle,
    pub(crate) code_table_handle: TableHandle,
    pub(crate) storage_table_handle: TableHandle,
}

impl<'a> IO<'a> {
    pub fn new(
        resolver: &'a dyn TableResolver,
        nonce_table_handle: TableHandle,
        balance_table_handle: TableHandle,
        code_table_handle: TableHandle,
        storage_table_handle: TableHandle,
    ) -> Self {
        Self {
            resolver,
            nonce_table_handle,
            balance_table_handle,
            code_table_handle,
            storage_table_handle,
        }
    }

    pub fn get_nonce(&self, address: &EthAddress) -> Option<U256> {
        let bytes = self
            .resolver
            .resolve_table_entry(&self.nonce_table_handle, &address.as_bytes())
            .unwrap();
        bytes.map(|bytes| read_u256_from_move_bytes(&bytes))
    }

    pub fn get_balance(&self, address: &EthAddress) -> Option<U256> {
        let bytes = self
            .resolver
            .resolve_table_entry(&self.balance_table_handle, &address.as_bytes())
            .unwrap();
        bytes.map(|bytes| read_u256_from_move_bytes(&bytes))
    }

    pub fn get_code(&self, address: &EthAddress) -> Vec<u8> {
        let bytes = self
            .resolver
            .resolve_table_entry(&self.code_table_handle, &address.as_bytes())
            .unwrap();
        bytes.unwrap_or_default()
    }

    pub fn get_storage(&self, address: &EthAddress, index: H256) -> Option<H256> {
        let storage_key = StorageKey::new(address.as_bytes().to_vec(), index.as_bytes().to_vec());
        let bytes = self
            .resolver
            .resolve_table_entry(&self.storage_table_handle, bcs::to_bytes(&storage_key).unwrap().as_slice())
            .unwrap();
        bytes.map(|bytes| read_h256_from_bytes(&bytes))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageKey {
    pub address: Vec<u8>,
    pub offset: Vec<u8>,
}

impl StorageKey {
    pub fn new(address: Vec<u8>, offset: Vec<u8>) -> Self {
        Self { address, offset }
    }
}
