/// This module implements a detached allowlist of delegators that are accepted into one's delegation pool.
/// Any account can edit their owned allowlist, but a delegation pool will only use the allowlist defined
/// under its owner's account.
module aptos_framework::delegation_pool_allowlist {

    use std::error;
    use std::features;
    use std::signer;
    use std::vector;

    use aptos_std::smart_table::{Self, SmartTable};

    use aptos_framework::event;

    /// Delegators allowlisting is not supported.
    const EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED: u64 = 1;

    /// Delegators allowlisting should be enabled to perform this operation.
    const EDELEGATORS_ALLOWLISTING_NOT_ENABLED: u64 = 2;

    /// Tracks a delegation pool's allowlist of delegators.
    /// A delegation pool will only use the allowlist defined under its owner's account.
    /// If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually
    /// evicted later by the pool owner.
    struct DelegationPoolAllowlisting has key, store {
        allowlist: SmartTable<address, bool>,
    }

    #[event]
    struct EnableDelegatorsAllowlisting has drop, store {
        owner_address: address,
    }

    #[event]
    struct DisableDelegatorsAllowlisting has drop, store {
        owner_address: address,
    }

    #[event]
    struct AllowlistDelegator has drop, store {
        owner_address: address,
        delegator_address: address,
    }

    #[event]
    struct RemoveDelegatorFromAllowlist has drop, store {
        owner_address: address,
        delegator_address: address,
    }

    #[view]
    /// Return whether allowlisting is enabled for the provided delegation pool owner.
    public fun allowlisting_enabled(owner_address: address): bool {
        exists<DelegationPoolAllowlisting>(owner_address)
    }

    #[view]
    /// Return whether the provided delegator is allowlisted.
    /// A delegator is allowlisted if:
    /// - allowlisting is disabled on the delegation pool's owner
    /// - delegator is part of the allowlist
    public fun delegator_allowlisted(
        owner_address: address,
        delegator_address: address,
    ): bool acquires DelegationPoolAllowlisting {
        if (!allowlisting_enabled(owner_address)) { return true };

        *smart_table::borrow_with_default(
            freeze(borrow_mut_delegators_allowlist(owner_address)),
            delegator_address,
            &false
        )
    }

    #[view]
    /// Return allowlist or revert if allowlisting is not enabled for the provided owner account.
    public fun get_delegators_allowlist(
        owner_address: address,
    ): vector<address> acquires DelegationPoolAllowlisting {
        assert_allowlisting_enabled(owner_address);

        let allowlist = vector[];
        smart_table::for_each_ref(freeze(borrow_mut_delegators_allowlist(owner_address)), |delegator, _included| {
            vector::push_back(&mut allowlist, *delegator);
        });
        allowlist
    }

    /// Enable delegators allowlisting as the pool owner.
    public entry fun enable_delegators_allowlisting(
        owner: &signer,
    ) {
        assert!(
            features::delegation_pool_allowlisting_enabled(),
            error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED)
        );

        let owner_address = signer::address_of(owner);
        if (allowlisting_enabled(owner_address)) { return };

        move_to(owner, DelegationPoolAllowlisting { allowlist: smart_table::new<address, bool>() });

        event::emit(EnableDelegatorsAllowlisting { owner_address });
    }

    /// Disable delegators allowlisting as the pool owner. The existing allowlist will be emptied.
    public entry fun disable_delegators_allowlisting(
        owner: &signer,
    ) acquires DelegationPoolAllowlisting {
        let owner_address = signer::address_of(owner);
        assert_allowlisting_enabled(owner_address);

        let DelegationPoolAllowlisting { allowlist } = move_from<DelegationPoolAllowlisting>(owner_address);
        // if the allowlist becomes too large, the owner can always remove some delegators
        smart_table::destroy(allowlist);

        event::emit(DisableDelegatorsAllowlisting { owner_address });
    }

    /// Allowlist a delegator as the pool owner.
    public entry fun allowlist_delegator(
        owner: &signer,
        delegator_address: address,
    ) acquires DelegationPoolAllowlisting {
        let owner_address = signer::address_of(owner);
        assert_allowlisting_enabled(owner_address);

        if (delegator_allowlisted(owner_address, delegator_address)) { return };

        smart_table::add(borrow_mut_delegators_allowlist(owner_address), delegator_address, true);

        event::emit(AllowlistDelegator { owner_address, delegator_address });
    }

    /// Remove a delegator from the allowlist as the pool owner.
    public entry fun remove_delegator_from_allowlist(
        owner: &signer,
        delegator_address: address,
    ) acquires DelegationPoolAllowlisting {
        let owner_address = signer::address_of(owner);
        assert_allowlisting_enabled(owner_address);

        if (!delegator_allowlisted(owner_address, delegator_address)) { return };

        smart_table::remove(borrow_mut_delegators_allowlist(owner_address), delegator_address);

        event::emit(RemoveDelegatorFromAllowlist { owner_address, delegator_address });
    }

    fun assert_allowlisting_enabled(owner_address: address) {
        assert!(allowlisting_enabled(owner_address), error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_ENABLED));
    }

    inline fun borrow_mut_delegators_allowlist(
        owner_address: address
    ): &mut SmartTable<address, bool> acquires DelegationPoolAllowlisting {
        &mut borrow_global_mut<DelegationPoolAllowlisting>(owner_address).allowlist
    }

    #[test_only]
    public fun enable_delegation_pool_allowlisting_feature(aptos_framework: &signer) {
        features::change_feature_flags(
            aptos_framework,
            vector[features::get_delegation_pool_allowlisting_feature()],
            vector[]
        );
    }

    #[test(owner = @0x123)]
    #[expected_failure(abort_code = 0x30001, location = Self)]
    public entry fun test_delegators_allowlisting_not_supported(
        owner: &signer,
    ) {
        enable_delegators_allowlisting(owner);
    }

    #[test(aptos_framework = @aptos_framework, owner = @0x123)]
    #[expected_failure(abort_code = 0x30002, location = Self)]
    public entry fun test_disable_delegators_allowlisting_failure_1(
        aptos_framework: &signer,
        owner: &signer,
    ) acquires DelegationPoolAllowlisting {
        enable_delegation_pool_allowlisting_feature(aptos_framework);
        let owner_address = signer::address_of(owner);

        assert!(!allowlisting_enabled(owner_address), 0);
        disable_delegators_allowlisting(owner);
    }

    #[test(aptos_framework = @aptos_framework, owner = @0x123, delegator = @0x234)]
    #[expected_failure(abort_code = 0x30002, location = Self)]
    public entry fun test_allowlist_delegator_failure_1(
        aptos_framework: &signer,
        owner: &signer,
        delegator: &signer,
    ) acquires DelegationPoolAllowlisting {
        enable_delegation_pool_allowlisting_feature(aptos_framework);
        let owner_address = signer::address_of(owner);

        assert!(!allowlisting_enabled(owner_address), 0);
        allowlist_delegator(owner, signer::address_of(delegator));
    }

    #[test(aptos_framework = @aptos_framework, owner = @0x123, delegator = @0x234)]
    #[expected_failure(abort_code = 0x30002, location = Self)]
    public entry fun test_remove_delegator_from_allowlist_failure_1(
        aptos_framework: &signer,
        owner: &signer,
        delegator: &signer,
    ) acquires DelegationPoolAllowlisting {
        enable_delegation_pool_allowlisting_feature(aptos_framework);
        let owner_address = signer::address_of(owner);

        assert!(!allowlisting_enabled(owner_address), 0);
        remove_delegator_from_allowlist(owner, signer::address_of(delegator));
    }

    #[test(aptos_framework = @aptos_framework, owner = @0x123, delegator_1 = @0x234, delegator_2 = @0x345)]
    public entry fun test_delegation_pool_allowlisting_e2e(
        aptos_framework: &signer,
        owner: &signer,
        delegator_1: &signer,
        delegator_2: &signer,
    ) acquires DelegationPoolAllowlisting {
        enable_delegation_pool_allowlisting_feature(aptos_framework);
        let owner_address = signer::address_of(owner);
        let delegator_1_address = signer::address_of(delegator_1);
        let delegator_2_address = signer::address_of(delegator_2);

        assert!(!allowlisting_enabled(owner_address), 0);
        // any address is allowlisted if allowlist not created
        assert!(delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(delegator_allowlisted(owner_address, delegator_2_address), 0);

        enable_delegators_allowlisting(owner);
        // no address is allowlisted when allowlist is empty
        assert!(!delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(!delegator_allowlisted(owner_address, delegator_2_address), 0);
        let allowlist = &get_delegators_allowlist(owner_address);
        assert!(vector::length(allowlist) == 0, 0);

        allowlist_delegator(owner, delegator_1_address);
        assert!(delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(!delegator_allowlisted(owner_address, delegator_2_address), 0);
        allowlist = &get_delegators_allowlist(owner_address);
        assert!(vector::length(allowlist) == 1 && vector::contains(allowlist, &delegator_1_address), 0);

        allowlist_delegator(owner, delegator_2_address);
        assert!(delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(delegator_allowlisted(owner_address, delegator_2_address), 0);
        allowlist = &get_delegators_allowlist(owner_address);
        assert!(vector::length(allowlist) == 2 &&
            vector::contains(allowlist, &delegator_1_address) &&
            vector::contains(allowlist, &delegator_2_address),
            0
        );

        remove_delegator_from_allowlist(owner, delegator_2_address);
        assert!(delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(!delegator_allowlisted(owner_address, delegator_2_address), 0);
        allowlist = &get_delegators_allowlist(owner_address);
        assert!(vector::length(allowlist) == 1 && vector::contains(allowlist, &delegator_1_address), 0);

        // destroy the allowlist constructed so far
        disable_delegators_allowlisting(owner);
        assert!(!allowlisting_enabled(owner_address), 0);
        assert!(delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(delegator_allowlisted(owner_address, delegator_2_address), 0);

        enable_delegators_allowlisting(owner);
        assert!(!delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(!delegator_allowlisted(owner_address, delegator_2_address), 0);

        allowlist_delegator(owner, delegator_2_address);
        assert!(!delegator_allowlisted(owner_address, delegator_1_address), 0);
        assert!(delegator_allowlisted(owner_address, delegator_2_address), 0);
        allowlist = &get_delegators_allowlist(owner_address);
        assert!(vector::length(allowlist) == 1 && vector::contains(allowlist, &delegator_2_address), 0);

        // allowlist does not ever have duplicates
        allowlist_delegator(owner, delegator_2_address);
        assert!(vector::length(&get_delegators_allowlist(owner_address)) == 1, 0);

        // no override of existing allowlist when enabling allowlisting again
        enable_delegators_allowlisting(owner);
        assert!(vector::length(&get_delegators_allowlist(owner_address)) == 1, 0);

        // nothing changes when trying to remove an inexistent delegator
        remove_delegator_from_allowlist(owner, delegator_1_address);
        assert!(vector::length(&get_delegators_allowlist(owner_address)) == 1, 0);
    }
}
