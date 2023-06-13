spec aptos_framework::resource_account {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create_resource_account(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
    ) {
        pragma verify = true;
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);

        requires source_addr != resource_addr;

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include account::exists_at(resource_addr) ==> account::CreateResourceAccountAbortsIf;
        include !account::exists_at(resource_addr) ==> account::CreateAccountAbortsIf {addr: resource_addr};

        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let account = global<account::Account>(source_addr);

        aborts_if get && !exists<account::Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && len(global<account::Account>(source_addr).authentication_key) != 32;
        aborts_if !get && len(optional_auth_key) != 32;
    }

    spec create_resource_account_and_fund(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
        fund_amount: u64,
    ) {
        pragma verify = true;
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let account_source = global<account::Account>(source_addr);
        let account_resource = global<account::Account>(resource_addr);

        requires source_addr != resource_addr;

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include account::exists_at(resource_addr) ==> account::CreateResourceAccountAbortsIf;
        include !account::exists_at(resource_addr) ==> account::CreateAccountAbortsIf {addr: resource_addr};

        //coin::register properties
        aborts_if account::exists_at(resource_addr) && !coin::is_account_registered<AptosCoin>(resource_addr) && account_resource.guid_creation_num + 2 > MAX_U64;
        aborts_if account::exists_at(resource_addr) && !coin::is_account_registered<AptosCoin>(resource_addr) && account_resource.guid_creation_num + 2 >= account::MAX_GUID_CREATION_NUM;

        //coin::withdraw properties
        let coin_store_source = global<coin::CoinStore<AptosCoin>>(source_addr);
        let balance = coin_store_source.coin.value;

        aborts_if !coin::is_account_registered<AptosCoin>(source_addr);
        aborts_if coin_store_source.frozen;
        aborts_if balance < fund_amount;

        //coin::deposit properties
        let coin_store_resource = global<coin::CoinStore<AptosCoin>>(resource_addr);
        aborts_if coin::is_account_registered<AptosCoin>(resource_addr) && coin_store_resource.frozen;

        aborts_if get && !exists<account::Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && len(global<account::Account>(source_addr).authentication_key) != 32;
        aborts_if !get && len(optional_auth_key) != 32;
    }

    spec create_resource_account_and_publish_package(
        origin: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        pragma verify = false;
        //TODO: Loop in code.spec
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        let optional_auth_key = ZERO_AUTH_KEY;

        requires source_addr != resource_addr;

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include account::exists_at(resource_addr) ==> account::CreateResourceAccountAbortsIf;
        include !account::exists_at(resource_addr) ==> account::CreateAccountAbortsIf {addr: resource_addr};

        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let account = global<account::Account>(source_addr);

        aborts_if get && !exists<account::Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && len(global<account::Account>(source_addr).authentication_key) != 32;
        aborts_if !get && len(optional_auth_key) != 32;
    }

    spec rotate_account_authentication_key_and_store_capability(
        origin: &signer,
        resource: signer,
        resource_signer_cap: account::SignerCapability,
        optional_auth_key: vector<u8>,
    ) {
        let resource_addr = signer::address_of(resource);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf {
        use aptos_framework::account::{Account};
        origin: signer;
        resource_addr: address;
        optional_auth_key: vector<u8>;

        let origin_addr = signer::address_of(origin);
        let container = global<Container>(origin_addr);
        let get = len(optional_auth_key) == 0;

        aborts_if get && !exists<Account>(origin_addr);
        aborts_if exists<Container>(origin_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && !(exists<Account>(resource_addr) && len(global<Account>(origin_addr).authentication_key) == 32);
        aborts_if !get && !(exists<Account>(resource_addr) && len(optional_auth_key) == 32);
    }

    spec retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ) : account::SignerCapability  {
        aborts_if !exists<Container>(source_addr);
        let resource_addr = signer::address_of(resource);

        let container = borrow_global_mut<Container>(source_addr);
        aborts_if !simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if !exists<account::Account>(resource_addr);
    }
}
