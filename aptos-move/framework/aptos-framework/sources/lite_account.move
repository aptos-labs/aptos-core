module aptos_framework::lite_account {
    use std::bcs;
    use std::error;
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_framework::create_signer;
    use aptos_framework::event;
    use aptos_framework::function_info::{Self, FunctionInfo};
    use aptos_framework::guid;
    use aptos_framework::guid::GUID;

    friend aptos_framework::account;
    friend aptos_framework::resource_account;
    friend aptos_framework::transaction_validation;
    #[test_only]
    friend aptos_framework::lite_account_tests;

    const EACCOUNT_EXISTENCE: u64 = 1;
    const ECANNOT_RESERVED_ADDRESS: u64 = 2;
    const ESEQUENCE_NUMBER_OVERFLOW: u64 = 3;
    const EMALFORMED_AUTHENTICATION_KEY: u64 = 4;
    const ENATIVE_AUTHENTICATOR_IS_NOT_USED: u64 = 5;
    const ECUSTOMIZED_AUTHENTICATOR_IS_NOT_USED: u64 = 6;
    const EAUTH_FUNCTION_SIGNATURE_MISMATCH: u64 = 7;
    const ENOT_OWNER: u64 = 8;

    const MAX_U64: u128 = 18446744073709551615;

    #[event]
    struct UpdateNativeAuthenticator has store, drop {
        account: address,
        old_auth_key: Option<vector<u8>>,
        new_auth_key: Option<vector<u8>>,
    }

    #[event]
    struct UpdateDispatchableAuthenticator has store, drop {
        account: address,
        old_auth_function: Option<FunctionInfo>,
        new_auth_function: Option<FunctionInfo>
    }

    #[resource_group(scope = address)]
    /// A shared resource group for storing new account resources together in storage.
    struct LiteAccountGroup {}

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// Resource representing an account object.
    struct Account has key {
        sequence_number: u64,
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// The native authenticator where the key is used for authenticator verification in native code.
    struct NativeAuthenticator has key, copy, drop {
        auth_key: Option<vector<u8>>,
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// The dispatchable authenticator that defines how to authenticates this account in the specified module.
    /// An integral part of Account Abstraction.
    struct DispatchableAuthenticator has key, copy, drop {
        auth_function: FunctionInfo
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// Legacy field from deprecated Account module.
    struct LegacyGUIDCreactionNumber has key {
        creation_number: u64,
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// Legacy field from deprecated Account module.
    struct LegacyRotationCapabilityOffer has key, drop { for: address }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountGroup)]
    /// Legacy field from deprecated Account module.
    struct LegacySignerCapabilityOffer has key, drop { for: address }

    /// Update native authenticator, FKA account rotation.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun update_native_authenticator(
        account: &signer,
        key: vector<u8>,
    ) acquires NativeAuthenticator {
        update_native_authenticator_impl(account, option::some(key));
    }

    /// Remove native authenticator so that this account could not be authenticated via native authenticator.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun remove_native_authenticator(
        account: &signer,
    ) acquires NativeAuthenticator {
        update_native_authenticator_impl(account, option::none())
    }

    /// Update dispatchable authenticator that enables account abstraction.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun update_dispatchable_authenticator(
        account: &signer,
        module_address: address,
        module_name: String,
        function_name: String,
    ) acquires DispatchableAuthenticator {
        update_dispatchable_authenticator_impl(
            account,
            option::some(function_info::new_function_info_from_address(module_address, module_name, function_name))
        );
    }

    /// Update dispatchable authenticator that disables account abstraction.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun remove_dispatchable_authenticator(
        account: &signer,
    ) acquires DispatchableAuthenticator {
        update_dispatchable_authenticator_impl(
            account,
            option::none(),
        );
    }

    public(friend) fun update_native_authenticator_impl(
        account: &signer,
        new_auth_key_option: Option<vector<u8>>,
    ) acquires NativeAuthenticator {
        let addr = signer::address_of(account);
        if (option::is_some(&new_auth_key_option)) {
            let new_auth_key = option::borrow(&new_auth_key_option);
            assert!(
                vector::length(new_auth_key) == 32,
                error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
            );
            let native_auth_key = bcs::to_bytes(&addr);
            if (exists<NativeAuthenticator>(addr)) {
                if (option::some(native_auth_key) == new_auth_key_option) {
                    let NativeAuthenticator { auth_key } = move_from<NativeAuthenticator>(addr);
                    event::emit(
                        UpdateNativeAuthenticator { account: addr, old_auth_key: auth_key, new_auth_key: new_auth_key_option }
                    );
                } else {
                    let current = &mut borrow_global_mut<NativeAuthenticator>(addr).auth_key;
                    if (*current != new_auth_key_option) {
                        event::emit(
                            UpdateNativeAuthenticator { account: addr, old_auth_key: *current, new_auth_key: new_auth_key_option }
                        );
                        *current = new_auth_key_option;
                    };
                }
            } else if (new_auth_key != &native_auth_key) {
                move_to(account, NativeAuthenticator { auth_key: new_auth_key_option });
                event::emit(
                    UpdateNativeAuthenticator {
                        account: addr, old_auth_key: option::some(
                            native_auth_key
                        ), new_auth_key: new_auth_key_option
                    }
                )
            };
        } else if (exists<NativeAuthenticator>(addr)) {
            let authenticator = borrow_global_mut<NativeAuthenticator>(addr);
            if (option::is_some(&authenticator.auth_key)) {
                event::emit(UpdateNativeAuthenticator {
                    account: addr,
                    old_auth_key: authenticator.auth_key,
                    new_auth_key: option::none()
                });
                authenticator.auth_key = option::none();
            };
        } else {
            event::emit(UpdateNativeAuthenticator {
                account: addr,
                old_auth_key: option::some(bcs::to_bytes(&addr)),
                new_auth_key: option::none()
            });
            move_to(account, NativeAuthenticator { auth_key: option::none() });
        };
    }

    public(friend) fun update_dispatchable_authenticator_impl(
        account: &signer,
        auth_function_option: Option<FunctionInfo>,
    ) acquires DispatchableAuthenticator {
        let account_address = signer::address_of(account);
        if (option::is_some(&auth_function_option)) {
            let auth_function = option::destroy_some(auth_function_option);
            let dispatcher_auth_function_info = function_info::new_function_info_from_address(
                @aptos_framework,
                string::utf8(b"lite_account"),
                string::utf8(b"dispatchable_authenticate"),
            );
            assert!(
                function_info::check_dispatch_type_compatibility(&dispatcher_auth_function_info, &auth_function),
                error::invalid_argument(EAUTH_FUNCTION_SIGNATURE_MISMATCH)
            );
            if (exists<DispatchableAuthenticator>(account_address)) {
                let current = &mut borrow_global_mut<DispatchableAuthenticator>(account_address).auth_function;
                if (*current != auth_function) {
                    event::emit(
                        UpdateDispatchableAuthenticator {
                            account: account_address,
                            old_auth_function: option::some(*current),
                            new_auth_function: option::some(auth_function)
                        }
                    );
                    *current = auth_function;
                }
            } else {
                move_to(account, DispatchableAuthenticator { auth_function });
                event::emit(
                    UpdateDispatchableAuthenticator {
                        account: account_address,
                        old_auth_function: option::none(),
                        new_auth_function: option::some(auth_function)
                    }
                );
            }
        } else if (exists<DispatchableAuthenticator>(account_address)) {
            let DispatchableAuthenticator { auth_function } = move_from<DispatchableAuthenticator>(account_address);
            event::emit(UpdateDispatchableAuthenticator {
                account: account_address,
                old_auth_function: option::some(auth_function),
                new_auth_function: option::none()
            });
        }
    }

    /// Publishes a lite `Account` resource under `new_address`. A ConstructorRef representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!account_resource_exists_at(new_address), error::already_exists(EACCOUNT_EXISTENCE));

        // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
            error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
        );
        create_signer::create_signer(new_address)
    }

    public(friend) fun create_account_unchecked(addr: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!account_resource_exists_at(addr), error::already_exists(EACCOUNT_EXISTENCE));
        create_signer::create_signer(addr)
    }

    public(friend) fun create_account_with_resource(new_address: address): signer {
        let new_account = create_account(new_address);
        move_to(
            &new_account,
            Account {
                sequence_number: 0,
            }
        );
        new_account
    }

    #[view]
    /// Return `true` if Account resource exists at this address.
    public fun account_resource_exists_at(addr: address): bool {
        exists<Account>(addr)
    }

    #[view]
    /// Return `true` if the account could be authenticated with native authenticator.
    public fun using_native_authenticator(addr: address): bool acquires NativeAuthenticator {
        !exists<NativeAuthenticator>(addr) || option::is_some(&borrow_global<NativeAuthenticator>(addr).auth_key)
    }

    #[view]
    /// Return `true` if the account is an abstracted account that can be authenticated with dispatchable move authenticator.
    public fun using_dispatchable_authenticator(addr: address): bool {
        exists<DispatchableAuthenticator>(addr)
    }

    #[view]
    /// Return the current sequence number.
    public fun get_sequence_number(addr: address): u64 acquires Account {
        if (account_resource_exists_at(addr)) {
            borrow_global<Account>(addr).sequence_number
        } else {
            0
        }
    }

    #[view]
    /// Return the current native authenticator. `None` means this authentication scheme is disabled.
    public fun native_authenticator(addr: address): Option<vector<u8>> acquires NativeAuthenticator {
        if (exists<NativeAuthenticator>(addr)) {
            borrow_global<NativeAuthenticator>(addr).auth_key
        } else {
            option::some(bcs::to_bytes(&addr))
        }
    }

    #[view]
    /// Return the current dispatchable authenticator move function info. `None` means this authentication scheme is disabled.
    public fun dispatchable_authenticator(addr: address): Option<FunctionInfo> acquires DispatchableAuthenticator {
        if (exists<DispatchableAuthenticator>(addr)) {
            option::some(
                borrow_global<DispatchableAuthenticator>(addr).auth_function
            )
        } else { option::none() }
    }

    /// Bump sequence number, which is only called by transaction_validation.move in apilogue for sequential transactions.
    public(friend) fun increment_sequence_number(addr: address) acquires Account {
        if (!account_resource_exists_at(addr)) {
            create_account_with_resource(addr);
        };
        let sequence_number = &mut borrow_global_mut<Account>(addr).sequence_number;

        assert!(
            (*sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_OVERFLOW)
        );
        *sequence_number = *sequence_number + 1;
    }

    /// The native function to dispatch customized move authentication function.
    native fun dispatchable_authenticate(
        account_address: address,
        signature: vector<u8>,
        function: &FunctionInfo
    );

    ///////////////////////////////////////////////////////////////////////////
    /// Methods only for compatibility with account module.
    ///////////////////////////////////////////////////////////////////////////

    public(friend) fun guid_creation_number(addr: address): u64 acquires LegacyGUIDCreactionNumber {
        if (exists<LegacyGUIDCreactionNumber>(addr)) {
            borrow_global<LegacyGUIDCreactionNumber>(addr).creation_number
        } else {
            0
        }
    }

    public(friend) fun create_guid(account: &signer): GUID acquires LegacyGUIDCreactionNumber {
        let addr = signer::address_of(account);
        if (!exists<LegacyGUIDCreactionNumber>(addr)) {
            move_to(account, LegacyGUIDCreactionNumber {
                creation_number: 0
            });
        };
        let number = &mut borrow_global_mut<LegacyGUIDCreactionNumber>(addr).creation_number;
        guid::create(addr, number)
    }

    public(friend) fun rotation_capability_offer(
        addr: address,
    ): Option<address> acquires LegacyRotationCapabilityOffer {
        if (exists<LegacyRotationCapabilityOffer>(addr)) {
            option::some(borrow_global<LegacyRotationCapabilityOffer>(addr).for)
        } else {
            option::none()
        }
    }

    public(friend) fun signer_capability_offer(
        addr: address,
    ): Option<address> acquires LegacySignerCapabilityOffer {
        if (exists<LegacySignerCapabilityOffer>(addr)) {
            option::some(borrow_global<LegacySignerCapabilityOffer>(addr).for)
        } else {
            option::none()
        }
    }

    public(friend) fun set_rotation_capability_offer(
        account: &signer,
        offeree: Option<address>
    ) acquires LegacyRotationCapabilityOffer {
        let addr = signer::address_of(account);
        if (option::is_some(&offeree)) {
            let offeree = option::destroy_some(offeree);
            if (exists<LegacyRotationCapabilityOffer>(addr)) {
                borrow_global_mut<LegacyRotationCapabilityOffer>(addr).for = offeree;
            } else {
                move_to(account, LegacyRotationCapabilityOffer { for: offeree })
            }
        } else if (exists<LegacyRotationCapabilityOffer>(addr)) {
            move_from<LegacyRotationCapabilityOffer>(addr);
        }
    }

    public(friend) fun set_signer_capability_offer(
        account: &signer,
        offeree: Option<address>
    ) acquires LegacySignerCapabilityOffer {
        let addr = signer::address_of(account);
        if (option::is_some(&offeree)) {
            let offeree = option::destroy_some(offeree);
            if (exists<LegacySignerCapabilityOffer>(addr)) {
                borrow_global_mut<LegacySignerCapabilityOffer>(addr).for = offeree;
            } else {
                move_to(account, LegacySignerCapabilityOffer { for: offeree })
            }
        } else if (exists<LegacySignerCapabilityOffer>(addr)) {
            move_from<LegacySignerCapabilityOffer>(addr);
        }
    }

    #[test_only]
    public fun create_account_for_test(new_address: address): signer {
        create_signer::create_signer(new_address)
    }

    #[test(aaron = @0xcafe)]
    entry fun test_native_authenticator(aaron: &signer) acquires NativeAuthenticator, Account {
        let addr = signer::address_of(aaron);
        assert!(!account_resource_exists_at(addr), 0);
        assert!(!using_dispatchable_authenticator(addr), 0);
        assert!(using_native_authenticator(addr), 0);
        assert!(!exists<NativeAuthenticator>(addr), 0);
        assert!(native_authenticator(addr) == option::some(bcs::to_bytes(&addr)), 0);
        assert!(get_sequence_number(addr) == 0, 0);
        update_native_authenticator(aaron, bcs::to_bytes(&@0x1));
        assert!(native_authenticator(addr) == option::some(bcs::to_bytes(&@0x1)), 0);
        assert!(exists<NativeAuthenticator>(addr), 0);
        remove_native_authenticator(aaron);
        assert!(native_authenticator(addr) == option::none(), 0);
        assert!(exists<NativeAuthenticator>(addr), 0);
        assert!(!using_native_authenticator(addr), 0);
        update_native_authenticator(aaron, bcs::to_bytes(&addr));
        assert!(!exists<NativeAuthenticator>(addr), 0);
        assert!(native_authenticator(addr) == option::some(bcs::to_bytes(&addr)), 0);
    }

    #[test(bob = @0xb0b)]
    entry fun test_dispatchable_authenticator(
        bob: &signer,
    ) acquires Account, DispatchableAuthenticator {
        let bob_addr = signer::address_of(bob);
        create_account_for_test(bob_addr);
        assert!(!using_dispatchable_authenticator(bob_addr), 0);
        assert!(get_sequence_number(bob_addr) == 0, 0);

        increment_sequence_number(bob_addr);
        assert!(get_sequence_number(bob_addr) == 1, 0);
        update_dispatchable_authenticator(
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
