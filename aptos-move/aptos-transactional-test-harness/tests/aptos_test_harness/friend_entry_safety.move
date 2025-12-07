//# init --addresses alice=0xf75daa73fc071f93593335eb9033da804777eb94491650dd3f095ce6f778acb6
//#      --private-keys alice=56a26140eb233750cd14fb168c3eb4bd0782b099cde626ec8aff7f3cceb6364f

//# publish --private-key alice
module alice::entry_points_safety {
    friend alice::test_friend_caller;

    // Error: public(friend) entry function is unsafe without attribute
    public(friend) entry fun unsafe_friend_entry() {
    }

    // Ok: public(friend) entry function allowed with attribute
    #[lint::allow_unsafe_friend_entry]
    public(friend) entry fun safe_friend_entry() {
    }

    // Ok: private entry function
    entry fun private_entry() {
    }

    // Ok: public entry function
    public entry fun public_entry() {
    }
    
    // Ok: non-entry public(friend) function
    public(friend) fun friend_non_entry() {
    }
}

