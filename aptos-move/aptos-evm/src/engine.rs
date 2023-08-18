// Copyright © Aptos Foundation

use crate::eth_address::EthAddress;
use crate::evm_backend::EVMBackend;
use crate::evm_io::{StorageKey, IO};
#[cfg(test)]
use crate::in_memory_storage::InMemoryTableResolver;
use crate::utils::u256_to_move_arr;
use aptos_table_natives::{TableChange, TableChangeSet, TableHandle, TableResolver};
use evm::backend::{Apply, Backend, MemoryAccount};
use evm::executor::stack::{MemoryStackState, PrecompileFn, StackExecutor, StackSubstateMetadata};
use evm_core::ExitReason;
use evm_runtime::Config;
use move_core_types::account_address::AccountAddress;
use move_core_types::effects::Op;
use primitive_types::{H160, H256, U256};
use sha3::{Digest, Keccak256};
use std::collections::BTreeMap;
use std::str::FromStr;
pub struct Engine<'a> {
    pub(crate) io: IO<'a>,
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
        let io = IO::new(
            resolver,
            nonce_table_handle.clone(),
            balance_table_handle.clone(),
            code_table_handle.clone(),
            storage_table_handle.clone(),
        );
        Self { io, origin }
    }

    pub fn transfer(
        &mut self,
        sender: H160,
        receiver: H160,
        value: U256,
    ) -> (ExitReason, Vec<u8>, TableChangeSet) {
        self.transact_call(sender, receiver, value, Vec::new(), u64::MAX, Vec::new())
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
        let backend = EVMBackend::new(self.io.clone(), self.origin.clone());

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
        let backend = EVMBackend::new(self.io.clone(), self.origin.clone());

        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles: BTreeMap<_, PrecompileFn> = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);
        let ret = executor.transact_create(caller, value, init_code, gas_limit, access_list);
        let (values, logs) = executor.into_state().deconstruct();

        let table_cs = self.into_change_set(values, backend);
        (ret.0, ret.1, table_cs)
    }

    pub fn view(
        &self,
        caller: H160,
        address: H160,
        value: U256,
        data: Vec<u8>,
    ) -> (ExitReason, Vec<u8>) {
        let config = Config::istanbul();
        let backend = EVMBackend::new(self.io.clone(), self.origin.clone());

        let metadata = StackSubstateMetadata::new(u64::MAX, &config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles: BTreeMap<_, PrecompileFn> = BTreeMap::new();
        let mut executor = StackExecutor::new_with_precompiles(state, &config, &precompiles);

        let (status, result) =
            executor.transact_call(caller, address, value, data, u64::MAX, Vec::new());
        (status, result)
    }

    fn modify_nonce(address: &EthAddress, nonce: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (
            address.as_bytes().to_vec(),
            Op::Modify(u256_to_move_arr(nonce).to_vec()),
        )
    }

    fn modify_balance(address: &EthAddress, balance: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (
            address.as_bytes().to_vec(),
            Op::Modify(u256_to_move_arr(balance).to_vec()),
        )
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
        (
            address.as_bytes().to_vec(),
            Op::New(u256_to_move_arr(nonce).to_vec()),
        )
    }

    fn add_balance(address: &EthAddress, balance: &U256) -> (Vec<u8>, Op<Vec<u8>>) {
        (
            address.as_bytes().to_vec(),
            Op::New(u256_to_move_arr(balance).to_vec()),
        )
    }

    fn add_code(address: &EthAddress, code: &Vec<u8>) -> (Vec<u8>, Op<Vec<u8>>) {
        (address.as_bytes().to_vec(), Op::New(code.clone()))
    }

    fn add_storage(address: &EthAddress, index: &H256, value: &H256) -> (Vec<u8>, Op<Vec<u8>>) {
        let mut buf = [0u8; 52];
        buf[..20].copy_from_slice(&address.as_bytes());
        buf[20..].copy_from_slice(&index.as_bytes());
        let storage_key = StorageKey::new(address.as_bytes().to_vec(), index.as_bytes().to_vec());
        (
            bcs::to_bytes(&storage_key).unwrap(),
            Op::New(value.as_bytes().to_vec()),
        )
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
        I: IntoIterator<Item = (H256, H256)>,
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
                        println!("Created a new account at address {:?}", address);
                        let cs = Self::add_nonce(&eth_addr, &basic.nonce);
                        nonce_change_set.insert(cs.0, cs.1);
                        let cs = Self::add_balance(&eth_addr, &basic.balance);
                        balance_change_set.insert(cs.0, cs.1);
                        if let Some(code) = code {
                            let cs = Self::add_code(&eth_addr, &code);
                            code_change_set.insert(cs.0, cs.1);
                        } else {
                            let cs = Self::add_code(&eth_addr, &vec![]);
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
                            let current_storage = self.io.get_storage(&eth_addr, index);
                            if current_storage.is_some() {
                                let cs = Self::modify_storage(&eth_addr, &index, &value);
                                storage_change_set.insert(cs.0, cs.1);
                            } else {
                                let cs = Self::add_storage(&eth_addr, &index, &value);
                                storage_change_set.insert(cs.0, cs.1);
                            }

                        }
                    }
                },
                Apply::Delete { address } => {
                    let eth_addr = EthAddress::new(address);
                    let cs = Self::delete_nonce(&eth_addr);
                    nonce_change_set.insert(cs.0, cs.1);
                    let cs = Self::delete_balance(&eth_addr);
                    balance_change_set.insert(cs.0, cs.1);
                    let cs = Self::delete_code(&eth_addr);
                    code_change_set.insert(cs.0, cs.1);
                    // TODO:: add support for deletiing the storage as well.
                },
            }
        }
        let nonce_table_change = TableChange::new(nonce_change_set);
        let balance_table_change = TableChange::new(balance_change_set);
        let code_table_change = TableChange::new(code_change_set);
        let storage_table_change = TableChange::new(storage_change_set);
        let mut changes = BTreeMap::new();
        changes.insert(self.io.nonce_table_handle, nonce_table_change);
        changes.insert(self.io.balance_table_handle, balance_table_change);
        changes.insert(self.io.code_table_handle, code_table_change);
        changes.insert(self.io.storage_table_handle, storage_table_change);

        TableChangeSet {
            new_tables: Default::default(),
            removed_tables: Default::default(),
            changes,
        }
    }
}

pub fn get_contract_address(sender: &H160, nonce: &U256) -> H160 {
    let mut stream = rlp::RlpStream::new_list(2);
    stream.append(sender);
    stream.append(nonce);
    H256::from_slice(Keccak256::digest(&stream.out()).as_slice()).into()
}

#[cfg(test)]
fn test_view_function() {
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

    fn add_memory_account(
        resolver: &mut InMemoryTableResolver,
        nonce_table_handle: &TableHandle,
        balance_table_handle: &TableHandle,
        code_table_handle: &TableHandle,
        storage_table_handle: &TableHandle,
        address: &EthAddress,
        account: MemoryAccount,
    ) {
        resolver.add_table_entry(
            nonce_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.nonce).to_vec(),
        );
        resolver.add_table_entry(
            balance_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.balance).to_vec(),
        );
        resolver.add_table_entry(code_table_handle, address.as_bytes().to_vec(), account.code);
        for (index, value) in account.storage {
            let mut buf = [0u8; 52];
            buf[..20].copy_from_slice(&address.as_bytes());
            buf[20..].copy_from_slice(&index.as_bytes());
            resolver.add_table_entry(
                storage_table_handle,
                buf.to_vec(),
                value.as_bytes().to_vec(),
            );
        }
    }

    let account1 = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: vec![],
    };

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &EthAddress::new(H160::from_str("0x1000000000000000000000000000000000000000").unwrap()),
        account1,
    );

    let account2 =  MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code:   hex::decode("6080604052602060005534801561001557600080fd5b50610150806100256000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100a1565b60405180910390f35b610073600480360381019061006e91906100ed565b61007e565b005b60008054905090565b8060008190555050565b6000819050919050565b61009b81610088565b82525050565b60006020820190506100b66000830184610092565b92915050565b600080fd5b6100ca81610088565b81146100d557600080fd5b50565b6000813590506100e7816100c1565b92915050565b600060208284031215610103576101026100bc565b5b6000610111848285016100d8565b9150509291505056fea2646970667358221220ff60a155b2dbc75c159d70fef272a9b375d200d2bab4f3c1c0e1a60c2501202c64736f6c63430008110033").unwrap(),
    };

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &EthAddress::new(H160::from_str("0xf000000000000000000000000000000000000000").unwrap()),
        account2,
    );

    let mut engine = Engine::new(
        &table_resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(H160::default()),
    );

    //     pragma solidity >=0.8.2 <0.9.0;
    //
    //     /**
    //      * @title Storage
    //      * @dev Store & retrieve value in a variable
    //      * @custom:dev-run-script ./scripts/deploy_with_ethers.ts
    //      */
    //     contract Storage {
    //
    //         uint256 number = 32;
    //
    //         /**
    //          * @dev Store value in variable
    //          * @param num value to store
    //          */
    //         function store(uint256 num) public {
    //         number = num;
    //     }
    //
    //     /**
    //      * @dev Return value
    //      * @return value of 'number'
    //      */
    //     function retrieve() public view returns (uint256){
    //         return number;
    //     }
    // }

    // let contract_address = get_contract_address(&H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
    //                                             &U256::one());
    // println!("Deploying the smart contract at address  {:?}",  contract_address);
    // // Deploy above smart contract
    // let (exit_reason, _, table_cs) = engine.transact_create(
    //     H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
    //     U256::zero(),
    //     hex::decode("6080604052602060005534801561001557600080fd5b50610150806100256000396000f3fe608060405234801561001057600080fd5b50600436106100365760003560e01c80632e64cec11461003b5780636057361d14610059575b600080fd5b610043610075565b60405161005091906100a1565b60405180910390f35b610073600480360381019061006e91906100ed565b61007e565b005b60008054905090565b8060008190555050565b6000819050919050565b61009b81610088565b82525050565b60006020820190506100b66000830184610092565b92915050565b600080fd5b6100ca81610088565b81146100d557600080fd5b50565b6000813590506100e7816100c1565b92915050565b600060208284031215610103576101026100bc565b5b6000610111848285016100d8565b9150509291505056fea2646970667358221220ff60a155b2dbc75c159d70fef272a9b375d200d2bab4f3c1c0e1a60c2501202c64736f6c63430008110033").unwrap(),
    //     u64::MAX,
    //     Vec::new(),
    // );
    //
    //0x2e64cec1
    // let (exit_reason, output, table_cs) = engine.transact_call(
    //     H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
    //     H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
    //     U256::zero(),
    // // calls store with 32
    // hex::decode("6057361d0000000000000000000000000000000000000000000000000000000000000020").unwrap(),
    // u64::MAX,
    // Vec::new(),
    // );

    //
    // 0x2e64cec1
    let (exit_reason, output) = engine.view(
        H160::from_str("0x1000000000000000000000000000000000000000").unwrap(),
        H160::from_str("0xf000000000000000000000000000000000000000").unwrap(),
        U256::zero(),
        // call to retrieve
        hex::decode("2e64cec1").unwrap(),
    );

    println!("output is {:?}", output);
    println!("exit_reason: {:?}", exit_reason);
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

    fn add_memory_account(
        resolver: &mut InMemoryTableResolver,
        nonce_table_handle: &TableHandle,
        balance_table_handle: &TableHandle,
        code_table_handle: &TableHandle,
        storage_table_handle: &TableHandle,
        address: &EthAddress,
        account: MemoryAccount,
    ) {
        resolver.add_table_entry(
            nonce_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.nonce).to_vec(),
        );
        resolver.add_table_entry(
            balance_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.balance).to_vec(),
        );
        resolver.add_table_entry(code_table_handle, address.as_bytes().to_vec(), account.code);
        for (index, value) in account.storage {
            let mut buf = [0u8; 52];
            buf[..20].copy_from_slice(&address.as_bytes());
            buf[20..].copy_from_slice(&index.as_bytes());
            resolver.add_table_entry(
                storage_table_handle,
                buf.to_vec(),
                value.as_bytes().to_vec(),
            );
        }
    }

    let account1 = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: hex::decode("6080604052348015600f57600080fd5b506004361060285760003560e01c80630f14a40614602d575b600080fd5b605660048036036020811015604157600080fd5b8101908080359060200190929190505050606c565b6040518082815260200191505060405180910390f35b6000806000905060005b83811015608f5760018201915080806001019150506076565b508091505091905056fea26469706673582212202bc9ec597249a9700278fe4ce78da83273cb236e76d4d6797b441454784f901d64736f6c63430007040033").unwrap(),
    };

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &EthAddress::new(H160::from_str("0x1000000000000000000000000000000000000000").unwrap()),
        account1,
    );

    let account2 = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: Vec::new(),
    };

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &EthAddress::new(H160::from_str("0xf000000000000000000000000000000000000000").unwrap()),
        account2,
    );

    let mut engine = Engine::new(
        &table_resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(H160::default()),
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
fn test_balance_transfer() {
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

    fn add_memory_account(
        resolver: &mut InMemoryTableResolver,
        nonce_table_handle: &TableHandle,
        balance_table_handle: &TableHandle,
        code_table_handle: &TableHandle,
        storage_table_handle: &TableHandle,
        address: &EthAddress,
        account: MemoryAccount,
    ) {
        resolver.add_table_entry(
            nonce_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.nonce).to_vec(),
        );
        resolver.add_table_entry(
            balance_table_handle,
            address.as_bytes().to_vec(),
            u256_to_move_arr(&account.balance).to_vec(),
        );
        resolver.add_table_entry(code_table_handle, address.as_bytes().to_vec(), account.code);
        for (index, value) in account.storage {
            let mut buf = [0u8; 52];
            buf[..20].copy_from_slice(&address.as_bytes());
            buf[20..].copy_from_slice(&index.as_bytes());
            resolver.add_table_entry(
                storage_table_handle,
                buf.to_vec(),
                value.as_bytes().to_vec(),
            );
        }
    }

    let faucet_address =
        EthAddress::new(H160::from_str("0x1000000000000000000000000000000000000000").unwrap());
    let code = hex::decode("6080604052348015600f57600080fd5b506004361060285760003560e01c80630f14a40614602d575b600080fd5b605660048036036020811015604157600080fd5b8101908080359060200190929190505050606c565b6040518082815260200191505060405180910390f35b6000806000905060005b83811015608f5760018201915080806001019150506076565b508091505091905056fea26469706673582212202bc9ec597249a9700278fe4ce78da83273cb236e76d4d6797b441454784f901d64736f6c63430007040033").unwrap();

    let faucet_account = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(10000000),
        storage: BTreeMap::new(),
        code: code.clone(),
    };

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &faucet_address,
        faucet_account,
    );

    let account2 = MemoryAccount {
        nonce: U256::one(),
        balance: U256::from(1),
        storage: BTreeMap::new(),
        code: Vec::new(),
    };

    let test_address =
        EthAddress::new(H160::from_str("0xf000000000000000000000000000000000000000").unwrap());

    add_memory_account(
        &mut table_resolver,
        &nonce_table_handle,
        &balance_table_handle,
        &code_table_handle,
        &storage_table_handle,
        &test_address,
        account2,
    );

    let mut engine = Engine::new(
        &table_resolver,
        nonce_table_handle,
        balance_table_handle,
        code_table_handle,
        storage_table_handle,
        EthAddress::new(H160::default()),
    );

    assert_eq!(
        engine.io.get_balance(&faucet_address),
        Some(U256::from(10000000))
    );
    assert_eq!(engine.io.get_code(&faucet_address), code);
    assert_eq!(engine.io.get_nonce(&faucet_address), Some(U256::one()));

    assert_eq!(engine.io.get_balance(&test_address), Some(U256::one()));

    let (exit_reason, _, table_cs) =
        engine.transfer(faucet_address.raw(), test_address.raw(), U256::one());
    // This doesn't work as the outout hasn't been updated yet.
    //assert_eq!(engine.io.get_balance(&test_address), Some(U256::from_str("1001").unwrap()));
    println!("exit_reason: {:?}", exit_reason);
    println!("Result: {:?}", table_cs);
}

#[cfg(test)]
mod tests {
    use crate::engine::{test_balance_transfer, test_contract_in_memory_table, test_view_function};
    #[test]
    fn test_balance_transfer_flow() {
        test_balance_transfer();
    }

    #[test]
    fn test_run_loop_contract_table() {
        test_contract_in_memory_table();
    }

    #[test]
    fn test_view() {
        test_view_function();
    }
}
