/// Reconfiguration meta-state resources and util functions.
module aptos_framework::reconfiguration_state {
    use std::error;
    use std::string;
    use aptos_std::copyable_any;
    use aptos_std::copyable_any::Any;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    friend aptos_framework::genesis;
    friend aptos_framework::reconfiguration;
    friend aptos_framework::stake;

    const ERECONFIG_NOT_IN_PROGRESS: u64 = 1;

    /// Reconfiguration drivers update this resources to notify other modules of some reconfiguration state.
    struct State has key {
        /// The state variant packed as an `Any`.
        /// Currently the variant type is one of the following.
        /// - `ReconfigStateInactive`
        /// - `ReconfigStateActive`
        variant: Any,
    }

    /// A state variant indicating no reconfiguration is in progress.
    struct StateInactive has copy, drop, store {}

    /// A state variant indicating a reconfiguration is in progress.
    struct StateActive has copy, drop, store {
        start_time_secs: u64,
    }

    public(friend) fun initialize(fx: &signer) {
        system_addresses::assert_aptos_framework(fx);
        if (!exists<State>(@aptos_framework)) {
            move_to(fx, State {
                variant: copyable_any::pack(StateInactive {})
            })
        }
    }

    public fun initialize_for_testing(fx: &signer) {
        initialize(fx)
    }

    /// Return whether the reconfiguration state is marked "in progress".
    public(friend) fun is_in_progress(): bool acquires State {
        if (!exists<State>(@aptos_framework)) {
            return false
        };

        let state = borrow_global<State>(@aptos_framework);
        let variant_type_name = *string::bytes(copyable_any::type_name(&state.variant));
        variant_type_name == b"0x1::reconfiguration_state::StateActive"
    }

    /// Mark the reconfiguration state "in progress" if it is currently "stopped".
    /// The current time is also recorded as the reconfiguration start time. (Some module, e.g., `stake.move`, needs this info).
    public(friend) fun try_mark_as_in_progress() acquires State {
        let state = borrow_global_mut<State>(@aptos_framework);
        let variant_type_name = *string::bytes(copyable_any::type_name(&state.variant));
        if (variant_type_name == b"0x1::reconfiguration_state::StateInactive") {
            state.variant = copyable_any::pack(StateActive {
                start_time_secs: timestamp::now_seconds()
            });
        };
    }

    /// Get the unix time when the currently in-progress reconfiguration started.
    /// Abort if the reconfiguration state is not "in progress".
    public(friend) fun start_time_secs(): u64 acquires State {
        let state = borrow_global<State>(@aptos_framework);
        let variant_type_name = *string::bytes(copyable_any::type_name(&state.variant));
        if (variant_type_name == b"0x1::reconfiguration_state::StateActive") {
            let active = copyable_any::unpack<StateActive>(state.variant);
            active.start_time_secs
        } else {
            abort(error::invalid_state(ERECONFIG_NOT_IN_PROGRESS))
        }
    }

    /// Called at the end of every reconfiguration to mark the state as "stopped".
    /// Abort if the current state is not "in progress".
    public(friend) fun mark_as_completed() acquires State {
        let state = borrow_global_mut<State>(@aptos_framework);
        let variant_type_name = *string::bytes(copyable_any::type_name(&state.variant));
        if (variant_type_name == b"0x1::reconfiguration_state::StateActive") {
            state.variant = copyable_any::pack(StateInactive {});
        } else {
            abort(error::invalid_state(ERECONFIG_NOT_IN_PROGRESS))
        }
    }

    #[test(fx = @aptos_framework)]
    fun basic(fx: &signer) acquires State {
        // Setip.
        timestamp::set_time_has_started_for_testing(fx);
        initialize(fx);

        // Initially no reconfig is in progress.
        assert!(!is_in_progress(), 1);

        // "try_start" should work.
        timestamp::fast_forward_seconds(123);
        try_mark_as_in_progress();
        assert!(is_in_progress(), 1);
        assert!(123 == start_time_secs(), 1);

        // Redundant `try_start` should be no-op.
        timestamp::fast_forward_seconds(1);
        try_mark_as_in_progress();
        assert!(is_in_progress(), 1);
        assert!(123 == start_time_secs(), 1);

        // A `finish` call should work when the state is marked "in progess".
        timestamp::fast_forward_seconds(10);
        mark_as_completed();
        assert!(!is_in_progress(), 1);
    }
}
