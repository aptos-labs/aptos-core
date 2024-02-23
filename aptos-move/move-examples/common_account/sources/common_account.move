/// A framework for sharing a single object as a shared account across multiple accounts.
///
/// This creates an object that can be used as a shared account, with the ability for the other accounts
/// to generate the object's signer.  The creator maintains the ability to add and remove accounts from access
/// to the object's signer.
module common_account::common_account {
    use std::error;
    use std::signer;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    use aptos_framework::aptos_account;
    use aptos_framework::object;
    use aptos_framework::object::{ExtendRef, Object};

    /// Missing the common account Management
    const ENO_MANAGEMENT_RESOURCE_FOUND: u64 = 1;
    /// Account is not on the allowlist
    const ENOT_ALLOWLISTED: u64 = 2;
    /// Signer isn't admin
    const ENOT_ADMIN: u64 = 3;

    /// Placeholder to use a SmartTable as a Set
    struct Empty has drop, store {}

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Contains the metadata for managing the account, particularly around adminstration.
    struct Management has key {
        /// An object extend ref to retrieve the signer of the object.
        extend_ref: ExtendRef,
        /// An ACL of all the allowed accounts to get the signer.
        allowlist: SmartTable<address, Empty>,
    }

    /// Creates a new common account by creating a resource account and storing the capability.
    public entry fun create(sender: &signer, seed: vector<u8>) {
        let constructor = object::create_named_object(sender, seed);
        let extend_ref = object::generate_extend_ref(&constructor);
        let object_signer = object::generate_signer(&constructor);
        let object_address = object::address_from_constructor_ref(&constructor);

        // Ensure the shared object is set up as an account
        aptos_account::create_account(object_address);
        move_to(
            &object_signer,
            Management {
                extend_ref,
                allowlist: smart_table::new(),
            },
        );
    }

    /// Add the other account to the management group.
    entry fun add_account(
        sender: &signer,
        common_account: Object<Management>,
        other: address,
    ) acquires Management {
        let management = assert_is_admin(sender, common_account);
        smart_table::add(&mut management.allowlist, other, Empty {});
    }

    /// Remove an account from the management group.
    entry fun remove_account(
        admin: &signer,
        common_account: Object<Management>,
        other: address,
    ) acquires Management {
        let management = assert_is_admin(admin, common_account);
        assert!(smart_table::contains(&management.allowlist, other), error::not_found(ENOT_ALLOWLISTED));
        smart_table::remove(&mut management.allowlist, other);
    }

    /// Generate a signer for the common_account if permissions allow.
    public fun acquire_signer(
        sender: &signer,
        common_account: Object<Management>,
    ): signer acquires Management {
        let sender_addr = signer::address_of(sender);

        let management = borrow_management(common_account);
        assert!(smart_table::contains(&management.allowlist, sender_addr), error::not_found(ENOT_ALLOWLISTED));
        object::generate_signer_for_extending(&management.extend_ref)
    }

    inline fun assert_is_admin(admin: &signer, common_account: Object<Management>): &mut Management {
        assert!(
            object::is_owner(common_account, signer::address_of(admin)),
            error::permission_denied(ENOT_ADMIN)
        );
        borrow_management(common_account)
    }

    inline fun borrow_management(common_account: Object<Management>): &mut Management {
        let common_address = object::object_address(&common_account);
        assert!(
            exists<Management>(common_address),
            error::not_found(ENO_MANAGEMENT_RESOURCE_FOUND),
        );
        borrow_global_mut<Management>(common_address)
    }

    #[test_only]
    const TEST_SEED: vector<u8> = b"";

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    public fun test_end_to_end(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        create(alice, TEST_SEED);
        let common_object = get_common_object(&alice_addr);
        add_account(alice, common_object, bob_addr);
        let common = acquire_signer(bob, common_object);
        assert!(signer::address_of(&common) == object::object_address(&common_object), 0);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60002, location = Self)]
    fun test_no_account_signer(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);

        create(alice, TEST_SEED);
        let common_object = get_common_object(&alice_addr);
        acquire_signer(bob, common_object);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60002, location = Self)]
    fun test_account_revoke_none(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        create(alice, TEST_SEED);
        let common_object = get_common_object(&alice_addr);
        remove_account(alice, common_object, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    fun test_account_revoke_acl(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        create(alice, TEST_SEED);
        let common_object = get_common_object(&alice_addr);
        add_account(alice, common_object, bob_addr);
        remove_account(alice, common_object, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_wrong_admin(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);

        create(alice, TEST_SEED);
        let common_object = get_common_object(&alice_addr);
        add_account(bob, common_object, bob_addr);
    }

    #[test_only]
    inline fun get_common_address(creator: &address): address {
        object::create_object_address(creator, TEST_SEED)
    }

    #[test_only]
    inline fun get_common_object(creator: &address): Object<Management> {
        let common_address = object::create_object_address(creator, TEST_SEED);
        object::address_to_object<Management>(common_address)
    }
}
