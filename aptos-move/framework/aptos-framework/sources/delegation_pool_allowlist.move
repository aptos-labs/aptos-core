module aptos_framework::delegation_pool_allowlist {

    use std::error;
    use std::features;
    use std::signer;
    use std::vector;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    use aptos_framework::event;

    friend aptos_framework::delegation_pool;

    /// Delegators allowlisting is not supported.
    const EDELEGATORS_ALLOWLISTING_NOT_SUPPORTED: u64 = 1;

    /// Delegators allowlisting should be enabled to perform this operation.
    const EDELEGATORS_ALLOWLISTING_NOT_ENABLED: u64 = 2;

    /// Tracks a delegation pool's allowlist of delegators.
    /// If allowlisting is enabled, existing delegators are not implicitly allowlisted and they can be individually
    /// evicted later by the pool owner.
    struct DelegationPoolAllowlisting has key {
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
    /// Return whether allowlisting is enabled for the provided delegation pool.
    public fun allowlisting_enabled(owner_address: address): bool {
        exists<DelegationPoolAllowlisting>(owner_address)
    }

    #[view]
    /// Return whether the provided delegator is allowlisted.
    /// A delegator is allowlisted if:
    /// - allowlisting is disabled on the pool
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
    /// Return allowlist or revert if allowlisting is not enabled for the provided delegation pool.
    public fun get_delegators_allowlist(
        owner_address: address,
    ): vector<address> acquires DelegationPoolAllowlisting {
        assert_allowlisting_enabled(owner_address);

        let allowlist = vector[];
        smart_table::for_each_ref(freeze(borrow_mut_delegators_allowlist(owner_address)), |delegator, included| {
            if (*included) {
                vector::push_back(&mut allowlist, *delegator);
            }
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

    /// Remove a delegator from the allowlist as the pool owner, but do not unlock their stake.
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

    public fun assert_allowlisting_enabled(owner_address: address) {
        assert!(allowlisting_enabled(owner_address), error::invalid_state(EDELEGATORS_ALLOWLISTING_NOT_ENABLED));
    }

    inline fun borrow_mut_delegators_allowlist(
        owner_address: address
    ): &mut SmartTable<address, bool> acquires DelegationPoolAllowlisting {
        &mut borrow_global_mut<DelegationPoolAllowlisting>(owner_address).allowlist
    }
}
