module aptos_framework::account_abstraction {
    use std::bcs;
    use std::hash;
    use aptos_std::from_bcs;

    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use aptos_std::ordered_map::{Self, OrderedMap};
    use aptos_std::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::create_signer;
    use aptos_framework::event;
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::object;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::system_addresses;
    use aptos_framework::permissioned_signer::{Self, is_permissioned_signer};
    #[test_only]
    use aptos_framework::account::create_account_for_test;
    #[test_only]
    use aptos_framework::auth_data;

    friend aptos_framework::transaction_validation;
    #[test_only]
    friend aptos_framework::account_abstraction_tests;

    const EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED: u64 = 1;
    const EFUNCTION_INFO_EXISTENCE: u64 = 2;
    const EAUTH_FUNCTION_SIGNATURE_MISMATCH: u64 = 3;
    const ENOT_MASTER_SIGNER: u64 = 4;
    const EINCONSISTENT_SIGNER_ADDRESS: u64 = 5;
    const EDEPRECATED_FUNCTION: u64 = 6;
    const EDOMAIN_AA_NOT_INITIALIZED: u64 = 6;

    /// domain_aa_account_address uses this for domain separation within its native implementation
    /// source is defined in Scheme enum in types/src/transaction/authenticator.rs
    const DOMAIN_ABSTRACTION_DERIVED_SCHEME: u8 = 5;

    const MAX_U64: u128 = 18446744073709551615;

    #[event]
    struct UpdateDispatchableAuthenticator has store, drop {
        account: address,
        update: vector<u8>,
        auth_function: FunctionInfo,
    }

    #[event]
    struct RemoveDispatchableAuthenticator has store, drop {
        account: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The dispatchable authenticator that defines how to authenticates this account in the specified module.
    /// An integral part of Account Abstraction.
    enum DispatchableAuthenticator has key, copy, drop {
        V1 { auth_functions: OrderedMap<FunctionInfo, bool> }
    }

    enum DomainRegisterValue has store {
        Empty,
    }

    /// The dispatchable domain-scoped authenticator, that defines how to authenticate
    enum DomainDispatchableAuthenticator has key {
        V1 { auth_functions: BigOrderedMap<FunctionInfo, DomainRegisterValue> }
    }

    #[view]
    /// Return `true` if the account is an abstracted account that can be authenticated with dispatchable move authenticator.
    public fun using_dispatchable_authenticator(addr: address): bool {
        exists<DispatchableAuthenticator>(resource_addr(addr))
    }

    #[view]
    /// Return the current dispatchable authenticator move function info. `None` means this authentication scheme is disabled.
    public fun dispatchable_authenticator(addr: address): Option<vector<FunctionInfo>> acquires DispatchableAuthenticator {
        let resource_addr = resource_addr(addr);
        if (exists<DispatchableAuthenticator>(resource_addr)) {
            option::some(
                ordered_map::keys(&borrow_global<DispatchableAuthenticator>(resource_addr).auth_functions)
            )
        } else { option::none() }
    }

    #[view]
    /// Return the account address corresponding to the given `account_identity`,
    /// for the domain account abstraction defined by the given function.
    public fun derive_domain_account_address_view(
        module_address: address,
        module_name: String,
        function_name: String,
        account_identity: vector<u8>
    ): address {
        derive_domain_account_address(
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            &account_identity,
        )
    }

    /// Return the account address corresponding to the given `account_identity`,
    /// for the domain account abstraction defined by the given function.
    /// TODO: probably worth creating some module with all these derived functions,
    /// and do computation/caching in rust to avoid recomputation, as we do for objects.
    public fun derive_domain_account_address(domain_func_info: FunctionInfo, account_identity: &vector<u8>): address {
        // using bcs serialized structs here - this allows for no need for separators.
        // Alternative would've been to create unique string, we would need to convert domain_func_info into string,
        // then authentication_key to hex, and then we need separators as well - like ::
        let bytes = bcs::to_bytes(&domain_func_info);
        bytes.append(bcs::to_bytes(account_identity));
        bytes.push_back(DOMAIN_ABSTRACTION_DERIVED_SCHEME);
        from_bcs::to_address(hash::sha3_256(bytes))
    }

    /// Add dispatchable authentication function that enables account abstraction via this function.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun add_authentication_function(
        account: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DispatchableAuthenticator {
        assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        update_dispatchable_authenticator_impl(
            account,
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            true
        );
    }

    /// Remove dispatchable authentication function that enables account abstraction via this function.
    /// dispatchable function needs to verify that signing_data.authenticator() is a valid signature of signing_data.digest().
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun remove_authentication_function(
        account: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DispatchableAuthenticator {
        assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        update_dispatchable_authenticator_impl(
            account,
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            false
        );
    }

    /// Remove dispatchable authenticator so that all dispatchable authentication functions will be removed as well.
    /// After calling this function, the account is not abstracted at all.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun remove_authenticator(
        account: &signer,
    ) acquires DispatchableAuthenticator {
        assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(account);
        let resource_addr = resource_addr(addr);
        if (exists<DispatchableAuthenticator>(resource_addr)) {
            move_from<DispatchableAuthenticator>(resource_addr);
            event::emit(RemoveDispatchableAuthenticator {
                account: addr,
            });
        };
    }

    /// Add dispatchable domain-scoped authentication function, that enables account abstraction via this function.
    /// This means all accounts within the domain can use it to authenticate, without needing an initialization (unlike non-domain AA).
    /// dispatchable function needs to verify two things:
    /// - that signing_data.domain_authenticator() is a valid signature of signing_data.digest() (just like regular AA)
    /// - that signing_data.domain_account_identity() is correct identity representing the authenticator
    ///   (missing this step would allow impersonation)
    ///
    /// Note: This is  public entry function, as it requires framework signer, and that can
    /// only be obtained as a part of the governance script.
    public entry fun register_domain_with_authentication_function(
        aptos_framework: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DomainDispatchableAuthenticator {
        system_addresses::assert_aptos_framework(aptos_framework);

        borrow_global_mut<DomainDispatchableAuthenticator>(@aptos_framework).auth_functions.add(
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            DomainRegisterValue::Empty,
        );
    }

    entry fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(
            aptos_framework,
            DomainDispatchableAuthenticator::V1 { auth_functions: big_ordered_map::new_with_config(0, 0, false) }
        );
    }

    inline fun resource_addr(source: address): address {
        object::create_user_derived_object_address(source, @aptos_fungible_asset)
    }

    fun update_dispatchable_authenticator_impl(
        account: &signer,
        auth_function: FunctionInfo,
        is_add: bool,
    ) acquires DispatchableAuthenticator {
        let addr = signer::address_of(account);
        let resource_addr = resource_addr(addr);
        let dispatcher_auth_function_info = function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b"account_abstraction"),
            string::utf8(b"dispatchable_authenticate"),
        );
        assert!(
            function_info::check_dispatch_type_compatibility(&dispatcher_auth_function_info, &auth_function),
            error::invalid_argument(EAUTH_FUNCTION_SIGNATURE_MISMATCH)
        );
        if (is_add) {
            if (!exists<DispatchableAuthenticator>(resource_addr)) {
                move_to(
                    &create_signer::create_signer(resource_addr),
                    DispatchableAuthenticator::V1 { auth_functions: ordered_map::new() }
                );
            };
            let current_map = &mut borrow_global_mut<DispatchableAuthenticator>(resource_addr).auth_functions;
            assert!(
                !current_map.contains(&auth_function),
                error::already_exists(EFUNCTION_INFO_EXISTENCE)
            );
            current_map.add(auth_function, true);
            event::emit(
                UpdateDispatchableAuthenticator {
                    account: addr,
                    update: b"add",
                    auth_function,
                }
            );
        } else {
            assert!(exists<DispatchableAuthenticator>(resource_addr), error::not_found(EFUNCTION_INFO_EXISTENCE));
            let current_map = &mut borrow_global_mut<DispatchableAuthenticator>(resource_addr).auth_functions;
            assert!(
                current_map.contains(&auth_function),
                error::not_found(EFUNCTION_INFO_EXISTENCE)
            );
            current_map.remove(&auth_function);
            event::emit(
                UpdateDispatchableAuthenticator {
                    account: addr,
                    update: b"remove",
                    auth_function,
                }
            );
            if (current_map.length() == 0) {
                remove_authenticator(account);
            }
        };
    }

    inline fun dispatchable_authenticator_internal(addr: address): &OrderedMap<FunctionInfo, bool> {
        assert!(using_dispatchable_authenticator(addr), error::not_found(EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED));
        &borrow_global<DispatchableAuthenticator>(resource_addr(addr)).auth_functions
    }

    inline fun dispatchable_domain_authenticator_internal(): &BigOrderedMap<FunctionInfo, DomainRegisterValue> {
        assert!(exists<DomainDispatchableAuthenticator>(@aptos_framework), error::not_found(EDOMAIN_AA_NOT_INITIALIZED));
        &borrow_global<DomainDispatchableAuthenticator>(@aptos_framework).auth_functions
    }

    fun authenticate(
        account: signer,
        func_info: FunctionInfo,
        signing_data: AbstractionAuthData,
    ): signer acquires DispatchableAuthenticator, DomainDispatchableAuthenticator {
        let master_signer_addr = signer::address_of(&account);

        if (signing_data.is_domain()) {
            assert!(master_signer_addr == derive_domain_account_address(func_info, signing_data.domain_account_identity()), error::invalid_state(EINCONSISTENT_SIGNER_ADDRESS));

            let func_infos = dispatchable_domain_authenticator_internal();
            assert!(func_infos.contains(&func_info), error::not_found(EFUNCTION_INFO_EXISTENCE));
        } else {
            let func_infos = dispatchable_authenticator_internal(master_signer_addr);
            assert!(ordered_map::contains(func_infos, &func_info), error::not_found(EFUNCTION_INFO_EXISTENCE));
        };

        function_info::load_module_from_function(&func_info);
        let returned_signer = dispatchable_authenticate(account, signing_data, &func_info);
        // Returned signer MUST represent the same account address. Otherwise, it may break the invariant of Aptos blockchain!
        assert!(
            master_signer_addr == permissioned_signer::address_of(&returned_signer),
            error::invalid_state(EINCONSISTENT_SIGNER_ADDRESS)
        );
        returned_signer
    }

    /// The native function to dispatch customized move authentication function.
    native fun dispatchable_authenticate(
        account: signer,
        signing_data: AbstractionAuthData,
        function: &FunctionInfo
    ): signer;

    #[test(bob = @0xb0b)]
    entry fun test_dispatchable_authenticator(
        bob: &signer,
    ) acquires DispatchableAuthenticator {
        let bob_addr = signer::address_of(bob);
        create_account_for_test(bob_addr);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
        add_authentication_function(
            bob,
            @aptos_framework,
            string::utf8(b"account_abstraction_tests"),
            string::utf8(b"test_auth")
        );
        assert!(using_dispatchable_authenticator(bob_addr), 0);
        remove_authenticator(bob);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
    }

    #[test(bob = @0xb0b)]
    #[expected_failure(abort_code = 0x30005, location = Self)]
    entry fun test_authenticate_function_returning_invalid_signer(
        bob: signer,
    ) acquires DispatchableAuthenticator, DomainDispatchableAuthenticator {
        let bob_addr = signer::address_of(&bob);
        create_account_for_test(bob_addr);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
        add_authentication_function(
            &bob,
            @aptos_framework,
            string::utf8(b"account_abstraction_tests"),
            string::utf8(b"invalid_authenticate")
        );
        let function_info = function_info::new_function_info_from_address(
            @aptos_framework,
            string::utf8(b"account_abstraction_tests"),
            string::utf8(b"invalid_authenticate")
        );
        authenticate(bob, function_info, auth_data::create_auth_data(vector[], vector[]));
    }

    #[deprecated]
    public entry fun add_dispatchable_authentication_function(
        _account: &signer,
        _module_address: address,
        _module_name: String,
        _function_name: String,
    ) {
        abort std::error::unavailable(EDEPRECATED_FUNCTION)
    }

    #[deprecated]
    public entry fun remove_dispatchable_authentication_function(
        _account: &signer,
        _module_address: address,
        _module_name: String,
        _function_name: String,
    ) {
        abort std::error::unavailable(EDEPRECATED_FUNCTION)
    }

    #[deprecated]
    public entry fun remove_dispatchable_authenticator(
        _account: &signer,
    ) {
        abort std::error::unavailable(EDEPRECATED_FUNCTION)
    }
}
