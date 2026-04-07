module 0x42::view_function_safety {
    struct State has key {
        val: u64
    }

    fun modify_state() acquires State {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
    }

    // Should warn: public view calling mutating helper
    #[view]
    public fun unsafe_view_indirect(): u64 acquires State {
        modify_state();
        1
    }

    // Should warn: public view directly using borrow_global_mut
    #[view]
    public fun unsafe_view_direct_borrow(): u64 acquires State {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
        1
    }

    // Should warn: public view using move_from
    #[view]
    public fun unsafe_view_move_from(): u64 acquires State {
        let State { val } = move_from<State>(@0x42);
        val
    }

    // Should warn: public view using move_from via helper
    fun helper_move_from(): u64 acquires State {
        let State { val } = move_from<State>(@0x42);
        val
    }

    #[view]
    public fun unsafe_view_move_from_indirect(): u64 acquires State {
        helper_move_from()
    }

    // Ok: suppressed with lint::skip
    #[view]
    #[lint::skip(mutable_view_function)]
    public fun suppressed_view(): u64 acquires State {
        modify_state();
        1
    }

    // Ok: private view can mutate (unusual but not the check's concern)
    #[view]
    fun private_view_mutates(): u64 acquires State {
        modify_state();
        1
    }

    // Ok: public view that only reads
    #[view]
    public fun safe_pure_view(): u64 acquires State {
        borrow_global<State>(@0x42).val
    }
}
