// Copyright © Aptos Foundation

use std::collections::BTreeMap;
use evm::executor::stack::{MemoryStackState, PrecompileFn, StackExecutor, StackSubstateMetadata};
use evm_core::ExitReason;
use evm_runtime::Config;
use primitive_types::{H160, H256, U256};
use aptos_table_natives::{TableChange, TableChangeSet, TableHandle, TableResolver};
use crate::eth_address::EthAddress;
use crate::evm_backend::{EVMBackend, StorageKey};
use move_core_types::account_address::AccountAddress;
use evm::backend::{Apply, Backend, Basic, MemoryAccount};
#[cfg(test)]
use crate::in_memory_storage::InMemoryTableResolver;
use crate::utils::{u256_to_move_arr};
use std::str::FromStr;
use move_core_types::effects::Op;

pub struct Engine<'a> {
    resolver: &'a dyn TableResolver,
    nonce_table_handle: TableHandle,
    balance_table_handle: TableHandle,
    code_table_handle: TableHandle,
    storage_table_handle: TableHandle,
    origin: EthAddress,
}

impl<'a> Engine<'a> {
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

    pub fn transact_call(
        &mut self,
        caller: H160,
        address: H160,
        value: U256,
        data: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> (ExitReason, Vec<u8>, TableChangeSet) {
        let config = Config::istanbul();
        let backend = EVMBackend::new(self.resolver,
                                      self.nonce_table_handle,
                                      self.balance_table_handle,
                                      self.code_table_handle,
                                      self.storage_table_handle,
                                      self.origin.clone());

        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles: BTreeMap<_, PrecompileFn> = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);
        let ret = executor.transact_call(caller, address, value, data, gas_limit, access_list);

        let (values, logs) = executor.into_state().deconstruct();
        let table_cs = self.into_change_set(values, backend);
        (ret.0, ret.1, table_cs)
    }


    pub fn transact_create(
        &mut self,
        caller: H160,
        value: U256,
        init_code: Vec<u8>,
        gas_limit: u64,
        access_list: Vec<(H160, Vec<H256>)>,
    ) -> (ExitReason, Vec<u8>, TableChangeSet) {
        let config = Config::istanbul();
        let backend = EVMBackend::new(self.resolver,
                                      self.nonce_table_handle,
                                      self.balance_table_handle,
                                      self.code_table_handle,
                                      self.storage_table_handle,
                                      self.origin.clone());

        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles: BTreeMap<_, PrecompileFn> = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);
        let ret = executor.transact_create(caller, value, init_code, gas_limit, access_list);
        let (values, logs) = executor.into_state().deconstruct();

        let table_cs = self.into_change_set(values, backend);
        (ret.0, ret.1, table_cs)
    }

    fn modify_nonce(address: &EthAddress, nonce: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Modify(u256_to_move_arr(nonce).to_vec()))
    }

    fn modify_balance(address: &EthAddress, balance: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Modify(u256_to_move_arr(balance).to_vec()))
    }

    fn modify_code(address: &EthAddress, code: &Vec<u8>) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Modify(code.clone()))
    }

    fn modify_storage(address: &EthAddress, index: &H256, value: &H256) -> (Vec<u8>, Op<Vec<u8>>) {
        let mut buf = [0u8; 52];
        buf[..20].copy_from_slice(&address.as_bytes());
        buf[20..].copy_from_slice(&index.as_bytes());
        (buf.to_vec(), Op::Modify(value.as_bytes().to_vec()))
    }

    fn add_nonce(address: &EthAddress, nonce: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::New(u256_to_move_arr(nonce).to_vec()))
    }

    fn add_balance(address: &EthAddress, balance: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::New(u256_to_move_arr(balance).to_vec()))
    }

    fn add_code(address: &EthAddress, code: &Vec<u8>) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::New(code.clone()))
    }

    fn add_storage(address: &EthAddress, index: &H256, value: &H256) -> (Vec<u8>, Op<Vec<u8>>) {
        let mut buf = [0u8; 52];
        buf[..20].copy_from_slice(&address.as_bytes());
        buf[20..].copy_from_slice(&index.as_bytes());
        let storage_key = StorageKey::new(address.as_bytes().to_vec(), index.as_bytes().to_vec());
        (bcs::to_bytes(&storage_key).unwrap(), Op::New(value.as_bytes().to_vec()))
    }

    fn delete_nonce(address: &EthAddress) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Delete)
    }

    fn delete_balance(address: &EthAddress) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Delete)
    }

    fn delete_code(address: &EthAddress) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::Delete)
    }

    fn into_change_set<A, I>(&self, values: A, backend: EVMBackend) -> TableChangeSet
        where
            A: IntoIterator<Item = Apply<I>>,
            I: IntoIterator<Item = (H256, H256)>
    {

        let mut nonce_change_set = BTreeMap::new();
        let mut balance_change_set = BTreeMap::new();
        let mut code_change_set = BTreeMap::new();
        let mut storage_change_set = BTreeMap::new();
        //println!("values: {:?}", values);
        for apply in values {
            match apply {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage,
                } => {
                    // println!("Apply::Modify: {:?}", address);
                    // println!("Apply::Modify: {:?}", basic);
                    // println!("Apply::Modify: {:?}", code);
                    let eth_addr = EthAddress::new(address);
                    if !backend.exists(address.clone()) {
                        let cs = Self::add_nonce(&eth_addr, &basic.nonce);
                        nonce_change_set.insert(cs.0, cs.1);
                        let cs = Self::add_balance(&eth_addr, &basic.balance);
                        balance_change_set.insert(cs.0, cs.1);
                        if let Some(code) = code {
                            let cs = Self::add_code(&eth_addr, &code);
                            code_change_set.insert(cs.0, cs.1);
                        }
                        for (index, value) in storage {
                            let cs = Self::add_storage(&eth_addr, &index, &value);
                            storage_change_set.insert(cs.0, cs.1);
                        }
                    } else {
                        let current_basic = backend.basic(address);
                        if current_basic.nonce != basic.nonce {
                            let cs = Self::modify_nonce(&eth_addr, &basic.nonce);
                            nonce_change_set.insert(cs.0, cs.1);
                        }
                        if current_basic.balance != basic.balance {
                            let cs = Self::modify_balance(&eth_addr, &basic.balance);
                            balance_change_set.insert(cs.0, cs.1);
                        }
                        if let Some(code) = code {
                            let cs = Self::modify_code(&eth_addr, &code);
                            code_change_set.insert(cs.0, cs.1);
                        }
                        for (index, value) in storage {
                            let cs = Self::modify_storage(&eth_addr, &index, &value);
                            storage_change_set.insert(cs.0, cs.1);
                        }
                    }
                }
                Apply::Delete { address } => {
                    let eth_addr = EthAddress::new(address);
                    let cs  = Self::delete_nonce(&eth_addr);
                    nonce_change_set.insert(cs.0, cs.1);
                    let cs = Self::delete_balance(&eth_addr);
                    balance_change_set.insert(cs.0, cs.1);
                    let cs = Self::delete_code(&eth_addr);
                    code_change_set.insert(cs.0, cs.1);
                    // TODO:: add support for deletiing the storage as well.
                }
            }
        }
        let nonce_table_change = TableChange::new( nonce_change_set);
        let balance_table_change = TableChange::new( balance_change_set);
        let code_table_change = TableChange::new( code_change_set);
        let storage_table_change = TableChange::new( storage_change_set);
        let mut changes = BTreeMap::new();
        changes.insert(self.nonce_table_handle, nonce_table_change);
        changes.insert(self.balance_table_handle, balance_table_change);
        changes.insert(self.code_table_handle, code_table_change);
        changes.insert(self.storage_table_handle, storage_table_change);

        TableChangeSet {
            new_tables: Default::default(),
            removed_tables: Default::default(),
            changes,
        }
    }

}

#[cfg(test)]
fn test_contract_in_memory_table() {
    let config = Config::istanbul();

    let mut table_resolver = InMemoryTableResolver::new();
    let nonce_table_handle = TableHandle(AccountAddress::random());
    table_resolver.add_table(nonce_table_handle.clone());
    let balance_table_handle = TableHandle(AccountAddress::random());
    table_resolver.add_table(balance_table_handle.clone());
    let code_table_handle = TableHandle(AccountAddress::random());
    table_resolver.add_table(code_table_handle.clone());
    let storage_table_handle = TableHandle(AccountAddress::random());
    table_resolver.add_table(storage_table_handle.clone());

    fn add_memory_account(resolver: &mut InMemoryTableResolver,
                          nonce_table_handle: &TableHandle,
                          balance_table_handle: &TableHandle,
                          code_table_handle: &TableHandle,
                          storage_table_handle: &TableHandle,
                          address: &EthAddress, account: MemoryAccount) {
        resolver.add_table_entry(nonce_table_handle, address.as_bytes().to_vec(), u256_to_move_arr( &account.nonce).to_vec());
        resolver.add_table_entry(balance_table_handle, address.as_bytes().to_vec(), u256_to_move_arr( &account.balance).to_vec());
        resolver.add_table_entry(code_table_handle, address.as_bytes().to_vec(), account.code);
        for (index, value) in account.storage {
            let mut buf = [0u8; 52];
            buf[..20].copy_from_slice(&address.as_bytes());
            buf[20..].copy_from_slice(&index.as_bytes());
            resolver.add_table_entry(storage_table_handle, buf.to_vec(), value.as_bytes().to_vec());
        }
    }

    let account1 = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: hex::decode("6080604052348015600f57600080fd5b506004361060285760003560e01c80630f14a40614602d575b600080fd5b605660048036036020811015604157600080fd5b8101908080359060200190929190505050606c565b6040518082815260200191505060405180910390f35b6000806000905060005b83811015608f5760018201915080806001019150506076565b508091505091905056fea26469706673582212202bc9ec597249a9700278fe4ce78da83273cb236e76d4d6797b441454784f901d64736f6c63430007040033").unwrap(),
    };

    add_memory_account(&mut table_resolver,
                       &nonce_table_handle,
                       &balance_table_handle,
                       &code_table_handle,
                       &storage_table_handle,
                       &EthAddress::new(H160::from_str("0x1000000000000000000000000000000000000000").unwrap(),),
                       account1);


    let account2 =  MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: Vec::new(),
    };

    add_memory_account(&mut table_resolver,
                       &nonce_table_handle,
                       &balance_table_handle,
                       &code_table_handle,
                       &storage_table_handle,
                       &EthAddress::new(H160::from_str("0xf000000000000000000000000000000000000000").unwrap()),
                       account2);

    let mut engine = Engine::new(&table_resolver,
                                 nonce_table_handle,
                                 balance_table_handle,
                                 code_table_handle,
                                 storage_table_handle,
                                 EthAddress::new(H160::default())
    );

    // let (_, _, table_cs) = engine.transact_call(
    //     H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
    //     H160::from_str("0x1000000000000000000000000000000000000000").unwrap(),
    //     U256::zero(),
    //     // hex::decode("0f14a4060000000000000000000000000000000000000000000000000000000000b71b00")
    //     // 	.unwrap(),
    //     hex::decode("0f14a4060000000000000000000000000000000000000000000000000000000000002ee0")
    //         .unwrap(),
    //     u64::MAX,
    //     Vec::new(),
    // );


    let (exit_reason, _, table_cs) = engine.transact_create(
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
        U256::zero(),
       hex::decode("608060405234801561001057600080fd5b506101e7806100206000396000f3fe608060405234801561001057600080fd5b506004361061004c5760003560e01c806306661abd14610051578063371303c01461006f5780636d4ce63c14610079578063b3bcfa8214610097575b600080fd5b6100596100a1565b60405161006691906100ff565b60405180910390f35b6100776100a7565b005b6100816100c2565b60405161008e91906100ff565b60405180910390f35b61009f6100cb565b005b60005481565b60016000808282546100b99190610149565b92505081905550565b60008054905090565b60016000808282546100dd919061017d565b92505081905550565b6000819050919050565b6100f9816100e6565b82525050565b600060208201905061011460008301846100f0565b92915050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b6000610154826100e6565b915061015f836100e6565b92508282019050808211156101775761017661011a565b5b92915050565b6000610188826100e6565b9150610193836100e6565b92508282039050818111156101ab576101aa61011a565b5b9291505056fea264697066735822122053546e6543071d1df5660aa17b6350143abd9a8d50b3783b39730ed27283673e64736f6c63430008120033").unwrap(),
        u64::MAX,
        Vec::new(),
    );

    println!("exit_reason: {:?}", exit_reason);
    println!("Result: {:?}", table_cs);
}


#[cfg(test)]
mod tests {
    use crate::engine::test_contract_in_memory_table;
    #[test]
    fn test_run_loop_contract_table() {
        test_contract_in_memory_table();
    }
}
