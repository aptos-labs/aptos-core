module aptos_framework::sched_txns_auth_num {
    /// We need this module outside of scheduled_txns to prevent cyclical dependency issues between
    /// `scheduled_txns module and account module` during `key rotation handling`

    use std::error;
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::system_addresses;

    friend aptos_framework::account;
    friend aptos_framework::scheduled_txns;

    /// Authorization number not found - must be initialized first via get_auth_num
    const EAUTH_NUM_NOT_FOUND: u64 = 1;

    /// Invalid signer - only framework can call this
    const EINVALID_SIGNER: u64 = 2;

    /// Stores the authorization number mapping per address
    struct AuthNumData has key {
        /// BigOrderedMap to track address -> current authorization number
        auth_num_map: BigOrderedMap<address, u64>
    }

    /// Initialize the authorization number map - called from scheduled_txns::initialize
    public fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);

        move_to(
            framework,
            AuthNumData { auth_num_map: big_ordered_map::new_with_reusable() }
        );
    }

    /// Returns the current authorization number for an address
    /// Lazy initialization: starts from 1 and stores in map upon first use
    public(friend) fun get_or_init_auth_num(addr: address): u64 acquires AuthNumData {
        let data = borrow_global_mut<AuthNumData>(@aptos_framework);
        if (data.auth_num_map.contains(&addr)) {
            *data.auth_num_map.borrow(&addr)
        } else {
            // Lazy initialization: start from 1
            let initial_auth_num = 1;
            data.auth_num_map.add(addr, initial_auth_num);
            initial_auth_num
        }
    }

    /// Returns the current authorization number for an address (read-only)
    /// Requires that the address already exists in auth_num_map (initialized via get_or_init_auth_num)
    public(friend) fun get_auth_num(addr: address): u64 acquires AuthNumData {
        let data = borrow_global<AuthNumData>(@aptos_framework);
        assert!(
            data.auth_num_map.contains(&addr),
            error::invalid_state(EAUTH_NUM_NOT_FOUND)
        );
        *data.auth_num_map.borrow(&addr)
    }

    /// Increments the authorization number for an address
    /// Requires that the address already exists in auth_num_map (initialized via get_or_init_auth_num)
    public(friend) fun increment_auth_num(addr: address) acquires AuthNumData {
        let data = borrow_global_mut<AuthNumData>(@aptos_framework);

        assert!(
            data.auth_num_map.contains(&addr),
            error::invalid_state(EAUTH_NUM_NOT_FOUND)
        );

        let current_auth_num = *data.auth_num_map.borrow(&addr);
        let new_auth_num = current_auth_num + 1;
        *data.auth_num_map.borrow_mut(&addr) = new_auth_num;
    }

    /// Handles key rotation by incrementing the authorization number
    /// Only increments if the address already exists in the auth_num_map
    public(friend) fun handle_key_rotation(addr: address) acquires AuthNumData {
        if (contains_addr(addr)) {
            increment_auth_num(addr);
        }
        // If sender doesn't exist, do nothing
    }

    /// Sets a specific authorization number for an address (useful for testing or migration)
    public(friend) fun set_auth_num(addr: address, auth_num: u64) acquires AuthNumData {
        let data = borrow_global_mut<AuthNumData>(@aptos_framework);
        if (data.auth_num_map.contains(&addr)) {
            *data.auth_num_map.borrow_mut(&addr) = auth_num;
        } else {
            data.auth_num_map.add(addr, auth_num);
        }
    }

    /// Checks if an address exists in the authorization number map
    fun contains_addr(addr: address): bool acquires AuthNumData {
        let data = borrow_global<AuthNumData>(@aptos_framework);
        data.auth_num_map.contains(&addr)
    }
}
