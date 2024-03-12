module aptos_framework::lite_account {
    use std::error;
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::{Self, String};
    use aptos_std::simple_map;
    use aptos_std::simple_map::SimpleMap;
    use aptos_framework::create_signer;
    use aptos_framework::event;
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::object;
    use aptos_framework::signing_data::SigningData;
    #[test_only]
    use aptos_framework::account::create_account_for_test;

    friend aptos_framework::transaction_validation;
    #[test_only]
    friend aptos_framework::lite_account_tests;

    const EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED: u64 = 1;
    const EFUNCTION_INFO_EXISTENCE: u64 = 2;
    const EAUTH_FUNCTION_SIGNATURE_MISMATCH: u64 = 3;
    const ENOT_MASTER_SIGNER: u64 = 4;

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
    struct DispatchableAuthenticator has key, copy, drop {
        auth_functions: SimpleMap<FunctionInfo, bool>
    }

    /// Update dispatchable authenticator that enables account abstraction.
    /// Note: it is a private entry function that can only be called directly from transaction.
    public entry fun add_dispatchable_authentication_function(
        account: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DispatchableAuthenticator {
        //assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        update_dispatchable_authenticator_impl(
            account,
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            true
        );
    }

    public entry fun remove_dispatchable_authentication_function(
        account: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DispatchableAuthenticator {
        //assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        update_dispatchable_authenticator_impl(
            account,
            function_info::new_function_info_from_address(module_address, module_name, function_name),
            false
        );
    }

    /// Update dispatchable authenticator that disables account abstraction.
    /// Note: it is a private entry function that can only be called directly from transaction.
    public entry fun remove_dispatchable_authenticator(
        account: &signer,
    ) acquires DispatchableAuthenticator {
        //assert!(!is_permissioned_signer(account), error::permission_denied(ENOT_MASTER_SIGNER));
        let addr = signer::address_of(account);
        let resource_addr = resource_addr(addr);
        if (exists<DispatchableAuthenticator>(resource_addr)) {
            move_from<DispatchableAuthenticator>(resource_addr);
            event::emit(RemoveDispatchableAuthenticator {
                account: addr,
            });
        };
    }

    inline fun resource_addr(source: address): address {
        object::create_user_derived_object_address(source, @aptos_fungible_asset)
    }

    public(friend) fun update_dispatchable_authenticator_impl(
        account: &signer,
        auth_function: FunctionInfo,
        is_add: bool,
    ) acquires DispatchableAuthenticator {
        let addr = signer::address_of(account);
        let resource_addr = resource_addr(addr);
            let dispatcher_auth_function_info = function_info::new_function_info_from_address(
                @aptos_framework,
                string::utf8(b"lite_account"),
                string::utf8(b"dispatchable_authenticate"),
            );
            assert!(
                function_info::check_dispatch_type_compatibility(&dispatcher_auth_function_info, &auth_function),
                error::invalid_argument(EAUTH_FUNCTION_SIGNATURE_MISMATCH)
            );
        if (is_add && !exists<DispatchableAuthenticator>(resource_addr)) {
                move_to(&create_signer::create_signer(resource_addr), DispatchableAuthenticator {
                    auth_functions: simple_map::new()
                });
            };
            if (exists<DispatchableAuthenticator>(resource_addr)) {
                let current_map = &mut borrow_global_mut<DispatchableAuthenticator>(resource_addr).auth_functions;
                if (is_add) {
                    assert!(!simple_map::contains_key(current_map, &auth_function), error::already_exists(EFUNCTION_INFO_EXISTENCE));
                    simple_map::add(current_map, auth_function, true);
                } else {
                    assert!(simple_map::contains_key(current_map, &auth_function), error::not_found(EFUNCTION_INFO_EXISTENCE));
                    simple_map::remove(current_map, &auth_function);
                };
                event::emit(
                    UpdateDispatchableAuthenticator {
                        account: addr,
                        update: if (is_add) {b"add"} else {b"remove"},
                        auth_function,
                    }
                );
                if (simple_map::length(current_map) == 0) {
                    remove_dispatchable_authenticator(account);
                }
            };
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
                simple_map::keys(&borrow_global<DispatchableAuthenticator>(resource_addr).auth_functions)
            )
        } else { option::none() }
    }

    inline fun dispatchable_authenticator_internal(addr: address): &SimpleMap<FunctionInfo, bool> {
        assert!(using_dispatchable_authenticator(addr), error::not_found(EDISPATCHABLE_AUTHENTICATOR_IS_NOT_USED));
        &borrow_global<DispatchableAuthenticator>(resource_addr(addr)).auth_functions
    }

    fun authenticate(
        account: signer,
        func_info: FunctionInfo,
        signing_data: SigningData,
    ): signer acquires DispatchableAuthenticator {
        let func_infos = dispatchable_authenticator_internal(signer::address_of(&account));
        assert!(simple_map::contains_key(func_infos, &func_info), error::not_found(EFUNCTION_INFO_EXISTENCE));
        function_info::load_module_from_function(&func_info);
        dispatchable_authenticate(account, signing_data, &func_info)
    }

    /// The native function to dispatch customized move authentication function.
    native fun dispatchable_authenticate(
        account: signer,
        signing_data: SigningData,
        function: &FunctionInfo
    ): signer;

    #[test(bob = @0xb0b)]
    entry fun test_dispatchable_authenticator(
        bob: &signer,
    ) acquires DispatchableAuthenticator {
        let bob_addr = signer::address_of(bob);
        create_account_for_test(bob_addr);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
        add_dispatchable_authentication_function(
            bob,
            @aptos_framework,
            string::utf8(b"lite_account_tests"),
            string::utf8(b"test_auth")
        );
        assert!(using_dispatchable_authenticator(bob_addr), 0);
        remove_dispatchable_authenticator(bob);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
    }
}
