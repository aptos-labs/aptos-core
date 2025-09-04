/// A framework for sharing a single resource account across multiple accounts.
///
/// This creates a resource account with the ability for other signified accounts the ability to
/// generate the resource accounts signer. Specifically, the creator maintains the ability to add
/// and remove new accounts that have access to the resource account signer.
module common_account::common_account {
    use std::error;
    use std::signer;

    use velor_std::simple_map::{Self, SimpleMap};

    use velor_framework::account::{Self, SignerCapability};

    /// Missing the common account Management
    const ENO_MANAGEMENT_RESOURCE_FOUND: u64 = 1;
    /// Missing CommonAccount
    const ENO_ACCOUNT_RESOURCE_FOUND: u64 = 2;
    /// Missing the common account Capability
    const ENO_CAPABILITY_FOUND: u64 = 3;
    /// Was not offered a capability
    const ENO_CAPABILITY_OFFERED: u64 = 4;
    /// Signer isn't admin
    const ENOT_ADMIN: u64 = 5;
    /// Found an address different than expected at the Capability
    const EUNEXPECTED_PARALLEL_ACCOUNT: u64 = 6;

    /// Contains the signer capability that generates the common account signer.
    struct CommonAccount has key {
        signer_cap: SignerCapability,
    }

    struct Empty has drop, store {}

    /// Contains the metadata for managing the account, particularly around adminstration.
    struct Management has key {
        /// Entity that adds and removes entities that can support this account.
        admin: address,
        /// An ACL that defines entities that have available, unclaimed capabilities to control
        /// this account.
        unclaimed_capabilities: SimpleMap<address, Empty>,
    }

    /// A revokable capability that is stored on a users account.
    struct Capability has drop, key {
        common_account: address,
    }

    /// Creates a new common account by creating a resource account and storing the capability.
    public entry fun create(sender: &signer, seed: vector<u8>) {
        let (resource_signer, signer_cap) = account::create_resource_account(sender, seed);

        move_to(
            &resource_signer,
            Management {
                admin: signer::address_of(sender),
                unclaimed_capabilities: simple_map::create(),
            },
        );

        move_to(&resource_signer, CommonAccount { signer_cap });
    }

    /// Add the other account to the list of accounts eligible to claim a capability for this
    /// common_account
    public entry fun add_account(
        sender: &signer,
        common_account: address,
        other: address,
    ) acquires Management {
        let management = assert_is_admin(sender, common_account);
        simple_map::add(&mut management.unclaimed_capabilities, other, Empty {});
    }

    /// Remove an account from the management group.
    public entry fun remove_account(
        admin: &signer,
        common_account: address,
        other: address,
    ) acquires Capability, Management {
        let management = assert_is_admin(admin, common_account);
        if (simple_map::contains_key(&management.unclaimed_capabilities, &other)) {
            simple_map::remove(&mut management.unclaimed_capabilities, &other);
        } else {
            assert!(exists<Capability>(other), error::not_found(ENO_CAPABILITY_FOUND));
            move_from<Capability>(other);
        }
    }

    /// Acquire the capability to use the signer capability for the common_account.
    public entry fun acquire_capability(
        sender: &signer,
        common_account: address,
    ) acquires Management {
        let sender_addr = signer::address_of(sender);

        let management = borrow_management(common_account);
        assert!(
            simple_map::contains_key(&management.unclaimed_capabilities, &sender_addr),
            error::not_found(ENO_CAPABILITY_OFFERED),
        );
        simple_map::remove(&mut management.unclaimed_capabilities, &sender_addr);

        move_to(sender, Capability { common_account });
    }

    /// Generate a signer for the common_account if permissions allow.
    public fun acquire_signer(
        sender: &signer,
        common_account: address,
    ): signer acquires Capability, CommonAccount, Management {
        let sender_addr = signer::address_of(sender);
        if (!exists<Capability>(sender_addr)) {
          acquire_capability(sender, common_account)
        };
        let capability = borrow_global<Capability>(sender_addr);

        assert!(
            capability.common_account == common_account,
            error::invalid_state(EUNEXPECTED_PARALLEL_ACCOUNT),
        );

        let resource = borrow_global<CommonAccount>(common_account);
        account::create_signer_with_capability(&resource.signer_cap)
    }

    inline fun assert_is_admin(admin: &signer, common_account: address): &mut Management {
        let management = borrow_management(common_account);
        assert!(
            signer::address_of(admin) == management.admin,
            error::permission_denied(ENOT_ADMIN),
        );
        management
    }

    inline fun borrow_management(common_account: address): &mut Management {
        assert!(
            exists<Management>(common_account),
            error::not_found(ENO_MANAGEMENT_RESOURCE_FOUND),
        );
        borrow_global_mut<Management>(common_account)
    }

    #[test_only]
    use std::vector;

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    public fun test_end_to_end(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, Management, CommonAccount {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        add_account(alice, common_addr, bob_addr);
        acquire_capability(bob, common_addr);
        let common = acquire_signer(bob, common_addr);
        assert!(signer::address_of(&common) == common_addr, 0);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public fun test_no_account_capability(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        acquire_capability(bob, common_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60001, location = Self)]
    public fun test_no_account_signer(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, CommonAccount, Management {
        let alice_addr = signer::address_of(alice);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        acquire_signer(bob, common_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60004, location = Self)]
    public fun test_account_no_capability(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        acquire_capability(bob, common_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x60003, location = Self)]
    public fun test_account_revoke_none(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        remove_account(alice, common_addr, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    public fun test_account_revoke_capability(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        add_account(alice, common_addr, bob_addr);
        acquire_capability(bob, common_addr);
        remove_account(alice, common_addr, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    public fun test_account_revoke_acl(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        add_account(alice, common_addr, bob_addr);
        remove_account(alice, common_addr, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x50005, location = Self)]
    public fun test_wrong_admin(
        alice: &signer,
        bob: &signer,
    ) acquires Management {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let common_addr = account::create_resource_address(&alice_addr, vector::empty());

        create(alice, vector::empty());
        add_account(bob, common_addr, bob_addr);
    }

    #[test(alice = @0xa11c3, bob = @0xb0b)]
    #[expected_failure(abort_code = 0x30006, location = Self)]
    public fun test_wrong_cap(
        alice: &signer,
        bob: &signer,
    ) acquires Capability, Management, CommonAccount {
        let alice_addr = signer::address_of(alice);
        let bob_addr = signer::address_of(bob);
        let alice_common_addr = account::create_resource_address(&alice_addr, vector::empty());
        let bob_common_addr = account::create_resource_address(&bob_addr, vector::empty());

        create(alice, vector::empty());
        create(bob, vector::empty());
        add_account(alice, alice_common_addr, bob_addr);
        acquire_capability(bob, alice_common_addr);
        acquire_signer(bob, bob_common_addr);
    }
}
