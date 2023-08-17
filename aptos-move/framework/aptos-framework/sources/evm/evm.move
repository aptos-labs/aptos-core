
module aptos_framework::evm {
    use std::signer;
    use std::vector;
    use std::error;
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};

    /// Aptos framework doesn't have ETH Data resource
    const ENO_ETH_DATA: u64 = 1;

    struct StorageKey has store, copy, drop {
        contract_address: vector<u8>,
        offset: vector<u8>,
    }

    struct EvmData has key {
        nonce: Table<vector<u8>, u256>,
        balance: Table<vector<u8>, u256>,
        code: Table<vector<u8>, vector<u8>>,
        storage: Table<StorageKey, vector<u8>>,
        pub_keys: Table<vector<u8>, address>,
    }

    public entry fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (exists<EvmData>(@aptos_framework)) {
            return;
        };
        move_to<EvmData>(aptos_framework, EvmData {
            nonce: table::new(),
            balance: table::new(),
            code: table::new(),
            storage: table::new(),
            pub_keys: table::new(),
        });
    }

    public entry fun create_account(eth_addr: vector<u8>, pub_key: address) acquires EvmData {
        // Make sure hash of pubkey is the same as eth_addr
        // Keccack256(pub_key) | (Truncate it by 160 bit) == eth_addr.value

        //TODO: How to borrow mut?
        let data_ref = borrow_global_mut<EvmData>(@aptos_framework);
        table::upsert(&mut data_ref.pub_keys, eth_addr, pub_key);
    }

    public entry fun create(caller: vector<u8>, payload: vector<u8>, signature: vector<u8>) acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        //TODO: How to borrow mut?
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        create_impl(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature)
    }

    public entry fun call(caller: vector<u8>, payload: vector<u8>, signature: vector<u8>) acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);

        call_impl(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature)
    }

    native fun create_impl(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, payload: vector<u8>, signature: vector<u8>);

    native fun call_impl(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, payload: vector<u8>, signature: vector<u8>);

    // native fun create_impl(caller: Vec<u8>, value: u256, init_code: Vec<u8>, gas_limit: u64);

    // native fun call_impl(caller: vector<u8>, address: vector<u8>, value: u256, data: vector<u8>, gas_limit: u64);
}
