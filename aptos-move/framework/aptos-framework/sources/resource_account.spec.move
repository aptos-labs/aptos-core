spec aptos_framework::resource_account {
    /// <high-level-req>
    /// No.: 1
    /// Requirement: The length of the authentication key must be 32 bytes.
    /// Criticality: Medium
    /// Implementation: The rotate_authentication_key_internal function ensures that the authentication key passed to it
    /// is of 32 bytes.
    /// Enforcement: Formally verified via [high-level-req-1](RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf).
    ///
    /// No.: 2
    /// Requirement: The Container structure must exist in the origin account in order to rotate the authentication key of
    /// a resource account and to store its signer capability.
    /// Criticality: High
    /// Implementation: The rotate_account_authentication_key_and_store_capability function makes sure the Container
    /// structure exists under the origin account.
    /// Enforcement: Formally verified via [high-level-req-2](rotate_account_authentication_key_and_store_capability).
    ///
    /// No.: 3
    /// Requirement: The resource account is registered for the Aptos coin.
    /// Criticality: High
    /// Implementation: The create_resource_account_and_fund ensures the newly created resource account is registered to
    /// receive the AptosCoin.
    /// Enforcement: Formally verified via [high-level-req-3](create_resource_account_and_fund).
    ///
    /// No.: 4
    /// Requirement: It is not possible to store two capabilities for the same resource address.
    /// Criticality: Medium
    /// Implementation: The rotate_account_authentication_key_and_store_capability will abort if the resource signer
    /// capability for the given resource address already exists in container.store.
    /// Enforcement: Formally verified via [high-level-req-4](rotate_account_authentication_key_and_store_capability).
    ///
    /// No.: 5
    /// Requirement: If provided, the optional authentication key is used for key rotation.
    /// Criticality: Low
    /// Implementation: The rotate_account_authentication_key_and_store_capability function will use optional_auth_key
    /// if it is provided as a parameter.
    /// Enforcement: Formally verified via [high-level-req-5](rotate_account_authentication_key_and_store_capability).
    ///
    /// No.: 6
    /// Requirement: The container stores the resource accounts' signer capabilities.
    /// Criticality: Low
    /// Implementation: retrieve_resource_account_cap will abort if there is no Container structure assigned to
    /// source_addr.
    /// Enforcement: Formally verified via [high-level-req-6](retreive_resource_account_cap).
    ///
    /// No.: 7
    /// Requirement: Resource account may retrieve the signer capability if it was previously added to its container.
    /// Criticality: High
    /// Implementation: retrieve_resource_account_cap will abort if the container of source_addr doesn't store the
    /// signer capability for the given resource.
    /// Enforcement: Formally verified via [high-level-req-7](retrieve_resource_account_cap).
    ///
    /// No.: 8
    /// Requirement: Retrieving the last signer capability from the container must result in the container being removed.
    /// Criticality: Low
    /// Implementation: retrieve_resource_account_cap will remove the container if the retrieved signer_capability was
    /// the last one stored under it.
    /// Enforcement: Formally verified via [high-level-req-8](retrieve_resource_account_cap).
    /// </high-level-req>
    ///
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec create_resource_account(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
    ) {
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
    }

    spec create_resource_account_and_fund(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
        fund_amount: u64,
    ) {
        use aptos_framework::aptos_account;
        // TODO(fa_migration)
        pragma verify = false;
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        let coin_store_resource = global<coin::CoinStore<AptosCoin>>(resource_addr);

        include aptos_account::WithdrawAbortsIf<AptosCoin>{from: origin, amount: fund_amount};
        include aptos_account::GuidAbortsIf<AptosCoin>{to: resource_addr};
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;

        //coin property
        aborts_if coin::spec_is_account_registered<AptosCoin>(resource_addr) && coin_store_resource.frozen;
        /// [high-level-req-3]
        ensures exists<aptos_framework::coin::CoinStore<AptosCoin>>(resource_addr);
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
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
    }

    spec rotate_account_authentication_key_and_store_capability(
        origin: &signer,
        resource: signer,
        resource_signer_cap: account::SignerCapability,
        optional_auth_key: vector<u8>,
    ) {
        let resource_addr = signer::address_of(resource);
        /// [high-level-req-1]
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;
        /// [high-level-req-2]
        ensures exists<Container>(signer::address_of(origin));
        /// [high-level-req-5]
        ensures vector::length(optional_auth_key) != 0 ==>
            global<aptos_framework::account::Account>(resource_addr).authentication_key == optional_auth_key;
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf {
        use aptos_framework::account::{Account};
        origin: signer;
        resource_addr: address;
        optional_auth_key: vector<u8>;

        let source_addr = signer::address_of(origin);
        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;

        aborts_if get && !exists<Account>(source_addr);
        /// [high-level-req-4]
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && !(exists<Account>(resource_addr) && len(global<Account>(source_addr).authentication_key) == 32);
        aborts_if !get && !(exists<Account>(resource_addr) && len(optional_auth_key) == 32);

        ensures simple_map::spec_contains_key(global<Container>(source_addr).store, resource_addr);
        ensures exists<Container>(source_addr);
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit {
        source_addr: address;
        optional_auth_key: vector<u8>;
        resource_addr: address;

        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let account = global<account::Account>(source_addr);

        aborts_if len(ZERO_AUTH_KEY) != 32;
        include account::exists_at(resource_addr) ==> account::CreateResourceAccountAbortsIf;
        include !account::exists_at(resource_addr) ==> account::CreateAccountAbortsIf {addr: resource_addr};

        aborts_if get && !exists<account::Account>(source_addr);
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && len(global<account::Account>(source_addr).authentication_key) != 32;
        aborts_if !get && len(optional_auth_key) != 32;

        ensures simple_map::spec_contains_key(global<Container>(source_addr).store, resource_addr);
        ensures exists<Container>(source_addr);
    }

    spec retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ) : account::SignerCapability  {
        /// [high-level-req-6]
        aborts_if !exists<Container>(source_addr);
        let resource_addr = signer::address_of(resource);

        let container = global<Container>(source_addr);
        /// [high-level-req-7]
        aborts_if !simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if !exists<account::Account>(resource_addr);
        /// [high-level-req-8]
        ensures simple_map::spec_contains_key(old(global<Container>(source_addr)).store, resource_addr) &&
            simple_map::spec_len(old(global<Container>(source_addr)).store) == 1 ==> !exists<Container>(source_addr);
        ensures exists<Container>(source_addr) ==> !simple_map::spec_contains_key(global<Container>(source_addr).store, resource_addr);
    }
}
