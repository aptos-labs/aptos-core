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
        // TODO: Could not verify `rotate_account_authentication_key_and_store_capability` because can't get `resource` and `resource_signer_cap`.
        pragma verify = false;
    }

    spec create_resource_account_and_fund(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
        fund_amount: u64,
    ) {
        // TODO: Could not verify `rotate_account_authentication_key_and_store_capability` because can't get `resource` and `resource_signer_cap`.
        pragma verify = false;
    }

    spec create_resource_account_and_publish_package(
        origin: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        // TODO: Calls `code::publish_package_txn`.
        // TODO: Could not verify `code::publish_package_txn` because can't get `resource` and `resource_signer_cap`.
        pragma verify = false;
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
