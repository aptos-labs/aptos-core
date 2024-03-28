/// Maintains the disallow list of the compiler ids for the blockchain.
module aptos_framework::compiler_id_config {
    use std::error;
    use std::signer;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    struct DisallowList has drop, key, store {
        disallow_list: vector<vector<u8>>,
    }

    /// Account is not authorized to make this change.
    const ENO_MESSAGE: u64 = 0;

    /// Only called during genesis.
    /// Publishes the DisallowList config.
    public(friend) fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);

        move_to(aptos_framework, DisallowList { disallow_list: vector[] });
    }

    /// Used in on-chain governances to update the disallow list.
    public entry fun set_disallow_list(account: &signer, disallow_list: vector<vector<u8>>) acquires DisallowList {
        system_addresses::assert_aptos_framework(account);
        assert!(exists<DisallowList>(signer::address_of(account)), error::not_found(ENO_MESSAGE));

        let config = borrow_global_mut<DisallowList>(@aptos_framework);
        config.disallow_list = disallow_list;

        // Need to trigger reconfiguration so validator nodes can sync on the updated disallow list.
        reconfiguration::reconfigure();
    }

    #[test(aptos_framework = @aptos_framework)]
    public entry fun test_set_version(aptos_framework: signer) acquires DisallowList {
        initialize(&aptos_framework);
        assert!(borrow_global<DisallowList>(@aptos_framework).disallow_list == vector[], 0);
        let new_ids = vector[b"id1"];
        set_disallow_list(&aptos_framework, new_ids);
        assert!(borrow_global<DisallowList>(@aptos_framework).disallow_list == new_ids, 1);
    }


    #[test(random_account = @0x123)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::system_addresses)]
    public entry fun test_initialize_unauthorized_should_fail(
        random_account: signer,
    ) {
        initialize(&random_account);
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 393216, location = Self)]
    public entry fun test_set_no_resource_should_fail(
        aptos_framework: signer,
    ) acquires DisallowList {
        set_disallow_list(&aptos_framework, vector[b"id1", b"id2"]);
    }

    #[test(random_account = @0x123)]
    #[expected_failure(abort_code = 327683, location = aptos_framework::system_addresses)]
    public entry fun test_set_unauthorized_should_fail(
        random_account: signer,
    ) acquires DisallowList {
        set_disallow_list(&random_account, vector[b"id1", b"id2"]);
    }
}
