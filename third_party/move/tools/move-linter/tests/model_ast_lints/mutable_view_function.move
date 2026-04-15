module 0x42::view_function_safety {
    struct State has key {
        val: u64
    }

    fun modify_state() {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
    }

    // Should warn: public view calling mutating helper
    #[view]
    public fun unsafe_view_indirect(): u64 {
        modify_state();
        1
    }

    // Should warn: public view directly using borrow_global_mut
    #[view]
    public fun unsafe_view_direct_borrow(): u64 {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
        1
    }

    // Should warn: public view using move_from
    #[view]
    public fun unsafe_view_move_from(): u64 {
        let State { val } = move_from<State>(@0x42);
        val
    }

    // Should warn: public view using move_from via helper
    fun helper_move_from(): u64 {
        let State { val } = move_from<State>(@0x42);
        val
    }

    #[view]
    public fun unsafe_view_move_from_indirect(): u64 {
        helper_move_from()
    }

    // Ok: suppressed with lint::skip
    #[view]
    #[lint::skip(mutable_view_function)]
    public fun suppressed_view(): u64 {
        modify_state();
        1
    }

    // Should warn: private view that mutates state
    #[view]
    fun private_view_mutates(): u64 {
        modify_state();
        1
    }

    // Ok: public view that only reads
    #[view]
    public fun safe_pure_view(): u64 {
        borrow_global<State>(@0x42).val
    }

    // Should warn: recursive function that modifies state, called by view
    fun recursive_mutating(n: u64) {
        if (n == 0) {
            let state = borrow_global_mut<State>(@0x42);
            state.val = state.val + 1;
        } else {
            recursive_mutating(n - 1);
        }
    }

    #[view]
    public fun unsafe_view_recursive(): u64 {
        recursive_mutating(5);
        1
    }

    // Ok: recursive function that does NOT modify state, called by view
    fun recursive_pure(n: u64): u64 {
        if (n == 0) {
            0
        } else {
            recursive_pure(n - 1) + 1
        }
    }

    #[view]
    public fun safe_view_recursive(): u64 {
        recursive_pure(5)
    }

    // Should warn: mutually recursive functions that modify state, called by view
    fun mutual_a(n: u64) {
        if (n == 0) {
            let state = borrow_global_mut<State>(@0x42);
            state.val = state.val + 1;
        } else {
            mutual_b(n - 1);
        }
    }

    fun mutual_b(n: u64) {
        mutual_a(n);
    }

    #[view]
    public fun unsafe_view_mutual_recursive(): u64 {
        mutual_a(3);
        1
    }

    fun z_mutates() {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
    }

    fun cycle_x() {
        cycle_y();
        z_mutates();
    }

    fun cycle_y() {
        cycle_x();
    }

    #[view]
    public fun first_view_reaches_x(): bool {
        cycle_x();
        true
    }

    // Should warn: y transitively mutates via x -> z_mutates
    #[view]
    public fun second_view_reaches_y(): bool {
        cycle_y();
        true
    }

    // Ok: mutually recursive functions that do NOT modify state, called by view
    fun mutual_pure_a(n: u64): u64 {
        if (n == 0) {
            0
        } else {
            mutual_pure_b(n - 1)
        }
    }

    fun mutual_pure_b(n: u64): u64 {
        mutual_pure_a(n) + 1
    }

    #[view]
    public fun safe_view_mutual_recursive(): u64 {
        mutual_pure_a(3)
    }

    // Should warn: deep transitive chain (view -> f -> g -> mutate)
    fun deep_level_3() {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
    }

    fun deep_level_2() {
        deep_level_3();
    }

    fun deep_level_1() {
        deep_level_2();
    }

    #[view]
    public fun unsafe_view_deep_chain(): u64 {
        deep_level_1();
        1
    }

}

module 0x42::friend_caller {}

module 0x42::view_friend_visibility {
    friend 0x42::friend_caller;

    struct State has key {
        val: u64
    }

    // Should warn: friend view that mutates state
    #[view]
    friend fun unsafe_friend_view_mutates(): u64 {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
        1
    }
}

module 0x42::view_package_visibility {
    struct State has key {
        val: u64
    }

    // Should warn: package view that mutates state
    #[view]
    package fun unsafe_package_view_mutates(): u64 {
        let state = borrow_global_mut<State>(@0x42);
        state.val = state.val + 1;
        1
    }
}
