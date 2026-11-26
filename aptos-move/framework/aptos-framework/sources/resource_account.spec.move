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
    /// Requirement: The resource account receives the funded Aptos coin.
    /// Criticality: High
    /// Implementation: When the transfer runs without dispatch hooks, create_resource_account_and_fund moves fund_amount
    /// into the resource account's paired APT primary store, creating the store if missing; if the origin has no such
    /// store, one is created empty and only a zero fund_amount succeeds.
    /// Enforcement: Formally verified via [high-level-req-3](create_resource_account_and_fund).
    ///
    /// No.: 4
    /// Requirement: It is not possible to store two capabilities for the same resource address.
    /// Criticality: Medium
    /// Implementation: The rotate_account_authentication_key_and_store_capability will abort if the resource signer
    /// capability for the given resource address already exists in container.store.
    /// Enforcement: Formally verified via [high-level-req-4](RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf).
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
    /// Enforcement: Formally verified via [high-level-req-6](retrieve_resource_account_cap).
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
        // Partial: the aborts of the coin transfer and of package publishing are not fully specified.
        pragma aborts_if_is_partial;
    }

    /// Creates the resource account at `spec_create_resource_address(origin, seed)` and stores its signer capability
    /// in the origin's `Container`.
    spec create_resource_account(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
    ) {
        pragma aborts_if_is_partial = false;
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
    }

    /// As `create_resource_account`, and moves `fund_amount` of `AptosCoin` from the origin to the resource account.
    spec create_resource_account_and_fund(
        origin: &signer,
        seed: vector<u8>,
        optional_auth_key: vector<u8>,
        fund_amount: u64,
    ) {
        let source_addr = signer::address_of(origin);
        let resource_addr = account::spec_create_resource_address(source_addr, seed);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit;
        include coin::AbortsIfNotExistCoinInfo<AptosCoin>;
        /// [high-level-req-3]
        include coin::TransferAbortsIfAndEnsures<AptosCoin> { from: source_addr, to: resource_addr, amount: fund_amount };
    }

    /// As `create_resource_account` with the zero authentication key, and publishes the package under the resource
    /// account.
    spec create_resource_account_and_publish_package(
        origin: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
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
        pragma aborts_if_is_partial = false;
        let resource_addr = signer::address_of(resource);
        let source_addr = signer::address_of(origin);
        include RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf;
        /// [high-level-req-2]
        ensures exists<Container>(signer::address_of(origin));
        ensures simple_map::spec_get(global<Container>(source_addr).store, resource_addr) == resource_signer_cap;
        /// [high-level-req-5]
        ensures vector::length(optional_auth_key) != 0 ==>
            global<aptos_framework::account::Account>(resource_addr).authentication_key == optional_auth_key;
        ensures vector::length(optional_auth_key) == 0 ==>
            global<aptos_framework::account::Account>(resource_addr).authentication_key
                == old(account::spec_get_authentication_key(source_addr));
    }

    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIf {
        origin: signer;
        resource_addr: address;
        optional_auth_key: vector<u8>;

        let source_addr = signer::address_of(origin);
        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;
        let auth_key = if (get) { account::spec_get_authentication_key(source_addr) } else { optional_auth_key };

        /// [high-level-req-4]
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && !account::spec_exists_at(source_addr);
        include account::EnsureResourceExistsAbortsIf { addr: resource_addr };
        /// [high-level-req-1]
        aborts_if len(auth_key) != 32;

        ensures simple_map::spec_contains_key(global<Container>(source_addr).store, resource_addr);
        ensures exists<Container>(source_addr);
    }

    /// The abort conditions and effects of creating a resource account and storing its signer capability.
    spec schema RotateAccountAuthenticationKeyAndStoreCapabilityAbortsIfWithoutAccountLimit {
        source_addr: address;
        optional_auth_key: vector<u8>;
        resource_addr: address;

        let container = global<Container>(source_addr);
        let get = len(optional_auth_key) == 0;

        // `account::create_resource_account`.
        aborts_if exists<account::Account>(resource_addr);
        include account::EnsureResourceExistsAbortsIf { addr: resource_addr };
        // Storing the capability and rotating the key.
        aborts_if exists<Container>(source_addr) && simple_map::spec_contains_key(container.store, resource_addr);
        aborts_if get && source_addr != resource_addr
            && len(account::spec_get_authentication_key(source_addr)) != 32;
        aborts_if !get && len(optional_auth_key) != 32;

        let post post_container = global<Container>(source_addr);
        ensures exists<account::Account>(resource_addr);
        ensures exists<Container>(source_addr);
        ensures simple_map::spec_contains_key(post_container.store, resource_addr);
        ensures simple_map::spec_get(post_container.store, resource_addr).account == resource_addr;
        ensures !get ==> global<account::Account>(resource_addr).authentication_key == optional_auth_key;
        ensures get && source_addr != resource_addr ==> global<account::Account>(resource_addr).authentication_key
            == old(account::spec_get_authentication_key(source_addr));
    }

    spec retrieve_resource_account_cap(
        resource: &signer,
        source_addr: address,
    ) : account::SignerCapability  {
        pragma aborts_if_is_partial = false;
        /// [high-level-req-6]
        aborts_if !exists<Container>(source_addr);
        let resource_addr = signer::address_of(resource);

        let container = global<Container>(source_addr);
        /// [high-level-req-7]
        aborts_if !simple_map::spec_contains_key(container.store, resource_addr);
        include account::EnsureResourceExistsAbortsIf { addr: resource_addr };
        /// [high-level-req-8]
        ensures simple_map::spec_contains_key(old(global<Container>(source_addr)).store, resource_addr) &&
            simple_map::spec_len(old(global<Container>(source_addr)).store) == 1 ==> !exists<Container>(source_addr);
        ensures exists<Container>(source_addr) ==> !simple_map::spec_contains_key(global<Container>(source_addr).store, resource_addr);
        // The other capabilities stay in the container.
        ensures simple_map::spec_len(old(global<Container>(source_addr)).store) > 1 ==> exists<Container>(source_addr)
            && global<Container>(source_addr).store
                == simple_map::spec_remove(old(global<Container>(source_addr)).store, resource_addr);
        ensures result == simple_map::spec_get(old(global<Container>(source_addr)).store, resource_addr);
        ensures global<account::Account>(resource_addr).authentication_key == ZERO_AUTH_KEY;
    }
}
