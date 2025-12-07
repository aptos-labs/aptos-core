//# init --addresses alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key alice
module alice::view_function_safety {
    
    struct State has key {
        val: u64,
    }

    fun modify_state() acquires State {
        let state = borrow_global_mut<State>(@alice);
        state.val = state.val + 1;
    }

    // Error: public view function modifying state via helper
    #[view]
    public fun unsafe_view_modifies_state() acquires State {
        modify_state();
    }

    // Error: public view function modifying state directly
    #[view]
    public fun unsafe_view_direct_borrow() acquires State {
        let state = borrow_global_mut<State>(@alice);
        state.val = state.val + 1;
    }

    // Ok: public view function modifying state allowed with attribute
    #[view]
    #[lint::allow_unsafe_mutable_view_function]
    public fun safe_view_modifies_state() acquires State {
        modify_state();
    }

    // Ok: private view function can modify state (though unusual)
    #[view]
    fun private_view_modifies_state() acquires State {
        modify_state();
    }

    // Ok: public view function reading state
    #[view]
    public fun safe_pure_view(): u64 acquires State {
        borrow_global<State>(@alice).val
    }
}

