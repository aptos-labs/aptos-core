// Copyright © Aptos Foundation

use evm_runtime::Config;
use aptos_table_natives::{TableHandle, TableResolver};
use evm::backend::{Backend, Basic, MemoryAccount, MemoryBackend, MemoryVicinity};
use primitive_types::{H160, H256, U256};
use std::collections::BTreeMap;
use evm::executor::stack::{MemoryStackState, StackExecutor, StackSubstateMetadata};
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::eth_address::EthAddress;
#[cfg(test)]
use crate::in_memory_storage::InMemoryTableResolver;
use crate::utils::{read_h256_from_bytes, read_u256_from_move_bytes};

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

pub struct EVMBackend<'a> {
    pub(crate) resolver: &'a dyn TableResolver,
    pub(crate) nonce_table_handle: TableHandle,
    pub(crate) balance_table_handle: TableHandle,
    pub(crate) code_table_handle: TableHandle,
    pub(crate) storage_table_handle: TableHandle,
    origin: EthAddress
}

impl<'a> EVMBackend<'a> {
    pub fn new(
        resolver: &'a dyn TableResolver,
        nonce_table_handle: TableHandle,
        balance_table_handle: TableHandle,
        code_table_handle: TableHandle,
        storage_table_handle: TableHandle,
        origin: EthAddress,
    ) -> Self {
        Self {
            resolver,
            nonce_table_handle,
            balance_table_handle,
            code_table_handle,
            storage_table_handle,
            origin,
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

impl<'a> Backend for EVMBackend<'a> {
    fn gas_price(&self) -> U256 {
        U256::from(100)
    }

    fn origin(&self) -> H160 {
        self.origin.raw()
    }

    fn block_hash(&self, _number: U256) -> H256 {
        todo!("block_hash not implemented")
    }

    /// Returns the current block index number.
    fn block_number(&self) -> U256 {
        todo!("block_number not implemented")
    }

    /// Returns a mocked coinbase which is the EVM address for the Aurora
    /// account, being 0x4444588443C3a91288c5002483449Aba1054192b.
    ///
    /// See: `https://doc.aurora.dev/develop/compat/evm#coinbase`
    fn block_coinbase(&self) -> H160 {
        H160([
            0x44, 0x44, 0x58, 0x84, 0x43, 0xC3, 0xA9, 0x12, 0x88, 0xC5, 0x00, 0x24, 0x83, 0x44,
            0x9A, 0xBA, 0x10, 0x54, 0x19, 0x2B,
        ])
    }

    /// Returns the current block timestamp.
    fn block_timestamp(&self) -> U256 {
        todo!("block_timestamp not implemented")
    }

    /// Returns the current block difficulty.
    ///
    /// See: `https://doc.aurora.dev/develop/compat/evm#difficulty`
    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    /// Get environmental block randomness.
    fn block_randomness(&self) -> Option<H256> {
        todo!("block_randomness not implemented")
    }

    /// Returns the current block gas limit.
    ///
    /// Currently, this returns 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
    /// as there isn't a gas limit alternative right now but this may change in
    /// the future.
    ///
    /// See: `https://doc.aurora.dev/develop/compat/evm#gaslimit`
    fn block_gas_limit(&self) -> U256 {
        U256::max_value()
    }

    /// Returns the current base fee for the current block.
    ///
    /// Currently, this returns 0 as there is no concept of a base fee at this
    /// time but this may change in the future.
    ///
    /// TODO: doc.aurora.dev link
    fn block_base_fee_per_gas(&self) -> U256 {
        U256::zero()
    }

    /// Returns the states chain ID.
    fn chain_id(&self) -> U256 {
        todo!("chain_id not implemented")
    }

    /// Checks if an address exists.
    fn exists(&self, address: H160) -> bool {
        let address = EthAddress::new(address);
        let nonce = self.get_nonce(&address);
        let balance = self.get_balance(&address);
        if !balance.is_none() || !nonce.is_none() {
            return true;
        }
        let code = self.get_code(&address);
        !code.is_empty()
    }

    /// Returns basic account information.
    fn basic(&self, address: H160) -> Basic {
        let address = EthAddress::new(address);
        let nonce = self.get_nonce(&address);
        let balance = self.get_balance(&address);
        Basic {
            nonce: nonce.unwrap_or_default(),
            balance: balance.unwrap_or_default(),
        }
    }

    /// Returns the code of the contract from an address.
    fn code(&self, address: H160) -> Vec<u8> {
        let address = EthAddress::new(address);
        self.get_code(&address)
    }

    /// Get storage value of address at index.
    fn storage(&self, address: H160, index: H256) -> H256 {
        let address = EthAddress::new(address);
        self.get_storage(&address, index).unwrap_or_default()
    }

    /// Get original storage value of address at index, if available.
    ///
    /// Since `SputnikVM` collects storage changes in memory until the transaction is over,
    /// the "original storage" will always be the same as the storage because no values
    /// are written to storage until after the transaction is complete.
    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        Some(self.storage(address, index))
    }
}

#[cfg(test)]
fn run_loop_contract_in_memory() {
    let config = Config::istanbul();

    let vicinity = MemoryVicinity {
        gas_price: U256::zero(),
        origin: H160::default(),
        block_hashes: Vec::new(),
        block_number: Default::default(),
        block_coinbase: Default::default(),
        block_timestamp: Default::default(),
        block_difficulty: Default::default(),
        block_gas_limit: Default::default(),
        chain_id: U256::one(),
        block_base_fee_per_gas: U256::zero(),
        block_randomness: None,
    };
    //
    let mut state = BTreeMap::new();
    state.insert(
        H160::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        MemoryAccount {
            nonce: U256::one(),
            balance: U256::from(10000000),
            storage: BTreeMap::new(),
            code: hex::decode("6080604052348015600f57600080fd5b506004361060285760003560e01c80630f14a40614602d575b600080fd5b605660048036036020811015604157600080fd5b8101908080359060200190929190505050606c565b6040518082815260200191505060405180910390f35b6000806000905060005b83811015608f5760018201915080806001019150506076565b508091505091905056fea26469706673582212202bc9ec597249a9700278fe4ce78da83273cb236e76d4d6797b441454784f901d64736f6c63430007040033").unwrap(),
        }
    );
    state.insert(
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
        MemoryAccount {
            nonce: U256::one(),
            balance: U256::from(10000000),
            storage: BTreeMap::new(),
            code: Vec::new(),
        },
    );

    let backend = MemoryBackend::new(&vicinity, state);
    let metadata = StackSubstateMetadata::new(u64::MAX, &config);
    let state = MemoryStackState::new(metadata, &backend);
    let precompiles = BTreeMap::new();
    let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);

    let _reason = executor.transact_call(
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
        H160::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        U256::zero(),
        // hex::decode("0f14a4060000000000000000000000000000000000000000000000000000000000b71b00")
        // 	.unwrap(),
        hex::decode("0f14a4060000000000000000000000000000000000000000000000000000000000002ee0")
            .unwrap(),
        u64::MAX,
        Vec::new(),
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_run_loop_contract_in_mem() {
        super::run_loop_contract_in_memory();
    }
}
