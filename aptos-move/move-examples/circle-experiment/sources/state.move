module message_transmitter::state {
    use aptos_framework::object::{Self, ExtendRef, spec_create_object_address};
    use std::signer;

    friend message_transmitter::message_transmitter;

    const ESTATE_DOES_NOT_EXIST: u64 = 1;
    const EINCORRECT_NONCE: u64 = 2;

    const SEED: vector<u8> = b"STATE";

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Management has key {
        extend_ref: ExtendRef
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct State has key {
        paused: bool,
        next_available_nonce: u64,
    }

    fun init_module(deployer: &signer) {
        let constructor_ref = &object::create_named_object(deployer, SEED);
        let object_signer = &object::generate_signer(constructor_ref);
        let extend_ref = object::generate_extend_ref(constructor_ref);

        move_to(object_signer, Management {
            extend_ref: extend_ref,
        });

        move_to(object_signer, State {
            paused: false,
            next_available_nonce: 0,
        });
    }

    public(friend) fun get_next_available_nonce(): u64 acquires State {
        let state = borrow_global<State>(get_signer_address());
        state.next_available_nonce
    }

    public(friend) fun set_next_available_nonce(nonce: u64) acquires State {
        let state = borrow_global_mut<State>(get_signer_address());
        state.next_available_nonce = nonce;
    }

    public(friend) fun get_signer_address(): address {
        object::create_object_address(&@message_transmitter, SEED)
    }

    public fun reserve_and_increment_nonce(): u64 acquires State {
        let nonce = get_next_available_nonce();
        set_next_available_nonce(nonce + 1);
        nonce
    }

    spec reserve_and_increment_nonce {
        let obj_addr = spec_create_object_address(@message_transmitter, SEED);
        aborts_if !exists<State>(obj_addr);
        aborts_if global<State>(obj_addr).next_available_nonce == MAX_U64;
        ensures global<State>(obj_addr).next_available_nonce == old(global<State>(obj_addr)).next_available_nonce + 1;
    }

    #[test_only]
    public fun init_for_test(message_transmitter: &signer) {
        init_module(message_transmitter);
    }

    #[test(message_transmitter = @0xcafe)]
    public fun test_state(message_transmitter: &signer) acquires State {
        init_for_test(message_transmitter);
        assert!(signer::address_of(message_transmitter) == @message_transmitter, ESTATE_DOES_NOT_EXIST);
        reserve_and_increment_nonce();
        assert!(get_next_available_nonce() == 1, EINCORRECT_NONCE);
    }
}
