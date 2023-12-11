module aptos_framework::lite_account {
    use std::bcs;
    use std::error;
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::create_signer;
    use aptos_framework::object;
    use aptos_framework::object::ObjectCore;

    friend aptos_framework::aptos_account;
    friend aptos_framework::resource_account;
    friend aptos_framework::transaction_validation;

    const EACCOUNT_EXISTENCE: u64 = 1;
    const ECANNOT_RESERVED_ADDRESS: u64 = 2;
    const ESEQUENCE_NUMBER_OVERFLOW: u64 = 3;
    const ENOT_OWNER: u64 = 4;
    const ENATIVE_AUTHENTICATOR_EXISTENCE: u64 = 5;

    const MAX_U64: u128 = 18446744073709551615;

    #[resource_group(scope = global)]
    /// A shared resource group for storing new account resources together in storage.
    struct LiteAccountResourceGroup {}

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountResourceGroup)]
    /// Resource representing an account object.
    struct Account has key {
        sequence_number: u64,
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountResourceGroup)]
    /// The native authenticator where the key is used for authenticator verification in native code.
    struct NativeAuthenticator has key, copy, store, drop {
        key: vector<u8>,
    }

    #[resource_group_member(group = aptos_framework::lite_account::LiteAccountResourceGroup)]
    /// The customized authenticator that defines how to authenticates this account in the specified module.
    /// An integral part of Account Abstraction.
    /// UNIMPLEMENTED.
    struct CustomizedAuthenticator has key, copy, store, drop {
        account_address: address,
        module_name: vector<u8>,
    }

    /// The function to generate native authenticator resource.
    public fun gen_native_authenticator(key: vector<u8>): NativeAuthenticator {
        NativeAuthenticator {
            key,
        }
    }

    /// The function to generate customized authenticator resource.
    public fun gen_customized_authenticator(
        account_address: address,
        module_name: vector<u8>,
    ): CustomizedAuthenticator {
        CustomizedAuthenticator { account_address, module_name }
    }

    /// Update native authenticator, FKA account rotation.
    /// Note: it is a private entry function that can only be called directly from transaction.
    entry fun update_native_authenticator(
        account: &signer,
        key: vector<u8>,
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        update_native_authenticator_impl(account, NativeAuthenticator {
            key,
        });
    }

    public(friend) fun update_native_authenticator_impl(
        account: &signer,
        authenticator: NativeAuthenticator
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        let account_address = signer::address_of(account);
        assert!(exists_at(account_address), error::not_found(EACCOUNT_EXISTENCE));
        if (exists<CustomizedAuthenticator>(account_address)) {
            move_from<CustomizedAuthenticator>(account_address);
        };
        if (exists<NativeAuthenticator>(account_address)) {
            let current = borrow_global_mut<NativeAuthenticator>(account_address);
            if (*current != authenticator) {
                *current = authenticator;
            }
        } else {
            move_to(account, authenticator)
        }
    }

    public(friend) fun update_customized_authenticator_impl(
        account: &signer,
        authenticator: CustomizedAuthenticator
    ) acquires CustomizedAuthenticator, NativeAuthenticator {
        let account_address = signer::address_of(account);
        assert!(exists_at(account_address), error::not_found(EACCOUNT_EXISTENCE));
        if (exists<NativeAuthenticator>(account_address)) {
            move_from<NativeAuthenticator>(account_address);
        };
        if (exists<CustomizedAuthenticator>(account_address)) {
            let current = borrow_global_mut<CustomizedAuthenticator>(account_address);
            if (*current != authenticator) {
                *current = authenticator;
            }
        } else {
            move_to(account, authenticator)
        }
    }

    /// Publishes a lite `Account` resource under `new_address`. A ConstructorRef representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!exists_at(new_address), error::already_exists(EACCOUNT_EXISTENCE));

        // NOTE: @core_resources gets created via a `create_account` call, so we do not include it below.
        assert!(
            new_address != @vm_reserved && new_address != @aptos_framework && new_address != @aptos_token,
            error::invalid_argument(ECANNOT_RESERVED_ADDRESS)
        );
        create_account_unchecked(new_address)
    }

    fun create_account_unchecked(new_address: address): signer {
        let new_account = create_signer::create_signer(new_address);
        move_to(
            &new_account,
            Account {
                sequence_number: 0,
            }
        );
        move_to(&new_account,
            NativeAuthenticator {
                key: bcs::to_bytes(&new_address)
            }
        );
        new_account
    }

    //entry fun rotate_account() {}

    #[view]
    public fun exists_at(addr: address): bool {
        account_resource_exists_at(addr) || (!account::exists_at(addr) && !object::object_exists<ObjectCore>(addr))
    }

    inline fun account_resource_exists_at(addr: address): bool {
        exists<Account>(addr)
    }

    #[view]
    public fun using_native_authenticator(addr: address): bool {
        exists_at(addr) && (exists<NativeAuthenticator>(addr) || !using_customized_authenticator(addr))
    }

    #[view]
    public fun using_customized_authenticator(addr: address): bool {
        exists<CustomizedAuthenticator>(addr)
    }

    #[view]
    public fun get_sequence_number(addr: address): u64 acquires Account {
        assert!(exists_at(addr), error::not_found(EACCOUNT_EXISTENCE));
        if (account_resource_exists_at(addr)) {
            borrow_global<Account>(addr).sequence_number
        } else {
            0
        }
    }

    #[view]
    public fun get_native_authentication_key(addr: address): vector<u8> acquires NativeAuthenticator {
        assert!(using_native_authenticator(addr), error::not_found(ENATIVE_AUTHENTICATOR_EXISTENCE));
        if (exists<NativeAuthenticator>(addr)) {
            borrow_global<NativeAuthenticator>(addr).key
        } else {
            bcs::to_bytes(&addr)
        }
    }

    // Only called by transaction_validation.move in apilogue for sequential transactions.
    public(friend) fun increment_sequence_number(addr: address) acquires Account {
        if (!account_resource_exists_at(addr)) {
            create_account(addr)
        };
        let sequence_number = &mut borrow_global_mut<Account>(addr).sequence_number;

        assert!(
            (*sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_OVERFLOW)
        );
        *sequence_number = *sequence_number + 1;
    }

    #[test_only]
    public fun create_account_for_test(new_address: address): signer {
        create_account_unchecked(new_address)
    }

    #[test(bob = @0xb0b)]
    fun test_account_basics(
        bob: &signer,
    ) acquires Account, CustomizedAuthenticator, NativeAuthenticator {
        let bob_addr = signer::address_of(bob);
        create_account_for_test(bob_addr);
        assert!(exists_at(bob_addr), 0);
        assert!(using_native_authenticator(bob_addr), 0);
        assert!(get_sequence_number(bob_addr) == 0, 0);

        increment_sequence_number(bob_addr);
        assert!(get_sequence_number(bob_addr) == 1, 0);

        update_customized_authenticator_impl(bob, gen_customized_authenticator(@aptos_framework, b"test"));
        assert!(using_customized_authenticator(bob_addr), 0);
    }
}
