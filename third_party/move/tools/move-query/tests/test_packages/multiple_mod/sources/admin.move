// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Admin module that is a friend of core.
module test_addr::admin {
    use test_addr::core::{Self, State};
    use test_addr::math;

    /// Admin capability struct.
    struct AdminCap has key, store {
        admin_address: address,
    }

    /// Creates admin capability.
    public fun create_admin_cap(admin: address): AdminCap {
        AdminCap { admin_address: admin }
    }

    /// Admin function to update state (uses friend access).
    public fun admin_update(state: &mut State, new_value: u64, _cap: &AdminCap) {
        core::set_value(state, new_value);
    }

    /// Admin function to increment state value.
    public fun admin_increment(state: &mut State, amount: u64, _cap: &AdminCap) {
        let current = core::get_value(state);
        let new_value = math::add(current, amount);
        core::set_value(state, new_value);
    }

    /// Admin function to initialize state (uses friend access).
    public fun admin_init(account: &signer, _cap: &AdminCap): State {
        core::initialize(account)
    }
}
