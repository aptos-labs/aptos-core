
module aptos_framework::evm {
    use std::signer;
    use std::vector;
    use std::error;
    use aptos_framework::system_addresses;
    use aptos_std::table::{Self, Table};
    use aptos_std::debug::print;

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

    public entry fun initialize(aptos_framework: &signer, eth_faucet_address: vector<u8>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (exists<EvmData>(@aptos_framework)) {
            return;
        };
        let balance = table::new();
        table::upsert(&mut balance, eth_faucet_address, 1000000000000);
        let nonce = table::new();
        table::upsert(&mut nonce, eth_faucet_address, 1);
        let code = table::new();
        table::upsert(&mut code, eth_faucet_address, vector::empty<u8>());
        move_to<EvmData>(aptos_framework, EvmData {
            nonce: nonce,
            balance: balance,
            code: code,
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
        create_impl(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature);
    }

    public entry fun call(caller: vector<u8>, payload: vector<u8>, signature: vector<u8>) acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        print(&data_ref.balance);
        call_impl(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature);
    }

    #[view]
    public fun view(caller: vector<u8>, payload: vector<u8>, signature: vector<u8>): vector<u8> acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);

        view_impl(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, payload, signature)
    }

    #[view]
    public fun view2(caller: vector<u8>, address: vector<u8>, value: vector<u8>, data: vector<u8>): vector<u8> acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);

        view_impl2(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, address, value, data)
    }

    native fun call_impl(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, payload: vector<u8>, signature: vector<u8>): vector<u8>;

    public entry fun create2(caller: vector<u8>, value: vector<u8>, init_code: vector<u8>, gas_limit: u64) acquires EvmData {

        //TODO: How to borrow mut?
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        create_impl2(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, value, init_code, gas_limit);
    }

    public entry fun call2(caller: vector<u8>, address: vector<u8>, value: vector<u8>, data: vector<u8>, gas_limit: u64) acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        print(&data_ref.balance);
        call_impl2(&data_ref.nonce, &data_ref.balance, &data_ref.code, &data_ref.storage, &data_ref.pub_keys, caller, address, value, data, gas_limit);
    }

    native fun view_impl(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, payload: vector<u8>, signature: vector<u8>): vector<u8>;

    native fun create_impl(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, payload: vector<u8>, signature: vector<u8>);

    #[view]
    public fun get_balance(caller: vector<u8>): u256 acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        *table::borrow(&data_ref.balance, caller)
    }

    #[view]
    public fun get_nonce(caller: vector<u8>): u256 acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        *table::borrow(&data_ref.nonce, caller)
    }

    #[view]
    public fun get_code(caller: vector<u8>): vector<u8> acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        *table::borrow(&data_ref.code, caller)
    }

    #[view]
    public fun get_pub_key(caller: vector<u8>): address acquires EvmData {
        assert!(
            exists<EvmData>(@aptos_framework),
            error::not_found(ENO_ETH_DATA),
        );
        let data_ref = borrow_global<EvmData>(@aptos_framework);
        *table::borrow(&data_ref.pub_keys, caller)
    }


    native fun create_impl2(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, value: vector<u8>, init_code: vector<u8>, gas_limit: u64): vector<u8>;

    native fun call_impl2(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, address: vector<u8>, value: vector<u8>, data: vector<u8>, gas_limit: u64) : vector<u8>;

    native fun view_impl2(nonce: &Table<vector<u8>, u256>, balance: &Table<vector<u8>, u256>, code: &Table<vector<u8>, vector<u8>>, storage: &Table<StorageKey, vector<u8>>, pub_keys: &Table<vector<u8>, address>, caller: vector<u8>, address: vector<u8>, value: vector<u8>, data: vector<u8>) : vector<u8>;
}
