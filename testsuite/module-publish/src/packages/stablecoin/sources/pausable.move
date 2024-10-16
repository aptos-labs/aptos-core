module stablecoin::pausable {
    use std::signer;
    use aptos_framework::event;
    use aptos_framework::object::{Self, Object};

    use stablecoin::ownable::{Self, OwnerRole};

    // === Errors ===

    /// Non-existent object.
    const ENON_EXISTENT_OBJECT: u64 = 0;
    /// Non-existent OwnerRole object.
    const ENON_EXISTENT_OWNER: u64 = 1;
    /// Caller is not pauser.
    const ENOT_PAUSER: u64 = 2;
    /// Object is paused.
    const EPAUSED: u64 = 3;

    // === Structs ===

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// The paused and pauser address state.
    struct PauseState has key {
        paused: bool,
        pauser: address
    }

    // === Events ===

    #[event]
    /// Emitted when PauseState paused is set to true.
    struct Pause has drop, store {
        obj_address: address
    }

    #[event]
    /// Emitted when PauseState paused is set to false.
    struct Unpause has drop, store {
        obj_address: address
    }

    #[event]
    /// Emitted when the PauseState pauser address is changed.
    struct PauserChanged has drop, store {
        obj_address: address,
        old_pauser: address,
        new_pauser: address
    }

    // === View-only functions ===

    #[view]
    /// Returns the PauseState pauser address.
    public fun pauser(obj: Object<PauseState>): address acquires PauseState {
        borrow_global<PauseState>(object::object_address(&obj)).pauser
    }

    #[view]
    /// Returns the PauseState paused status.
    public fun is_paused(obj: Object<PauseState>): bool acquires PauseState {
        borrow_global<PauseState>(object::object_address(&obj)).paused
    }

    /// Asserts that state is not paused.
    public fun assert_not_paused(obj_address: address) acquires PauseState {
        assert!(!is_paused(object::address_to_object<PauseState>(obj_address)), EPAUSED);
    }

    // === Write functions ===

    /// Creates and inits a new unpaused PauseState.
    public fun new(obj_signer: &signer, pauser: address) {
        let obj_address = signer::address_of(obj_signer);
        assert!(object::is_object(obj_address), ENON_EXISTENT_OBJECT);
        assert!(object::object_exists<OwnerRole>(obj_address), ENON_EXISTENT_OWNER);
        move_to(obj_signer, PauseState { paused: false, pauser });
    }

    /// Change the PauseState paused to true.
    entry fun pause(caller: &signer, obj: Object<PauseState>) acquires PauseState {
        let obj_address = object::object_address(&obj);
        let pause_state = borrow_global_mut<PauseState>(obj_address);
        assert!(pause_state.pauser == signer::address_of(caller), ENOT_PAUSER);
        if (!pause_state.paused) {
            pause_state.paused = true;
        };

        event::emit(Pause { obj_address });
    }

    /// Change the PauseState paused to false.
    entry fun unpause(caller: &signer, obj: Object<PauseState>) acquires PauseState {
        let obj_address = object::object_address(&obj);
        let pause_state = borrow_global_mut<PauseState>(obj_address);
        assert!(pause_state.pauser == signer::address_of(caller), ENOT_PAUSER);
        if (pause_state.paused) {
            pause_state.paused = false;
        };

        event::emit(Unpause { obj_address });
    }

    /// Change the PauseState pauser address.
    entry fun update_pauser(caller: &signer, obj: Object<PauseState>, new_pauser: address) acquires PauseState {
        let obj_address = object::object_address(&obj);
        ownable::assert_is_owner(caller, obj_address);
        let pause_state = borrow_global_mut<PauseState>(obj_address);
        let old_pauser = pause_state.pauser;

        pause_state.pauser = new_pauser;

        event::emit(PauserChanged { obj_address, old_pauser, new_pauser })
    }

    // === Test-only ===

    #[test_only]
    public fun test_Pause_event(obj_address: address): Pause {
        Pause { obj_address }
    }

    #[test_only]
    public fun test_Unpause_event(obj_address: address): Unpause {
        Unpause { obj_address }
    }

    #[test_only]
    public fun test_PauserChanged_event(
        obj_address: address, new_pauser: address, old_pauser: address
    ): PauserChanged {
        PauserChanged { obj_address, new_pauser, old_pauser }
    }

    #[test_only]
    public fun test_pause(caller: &signer, obj: Object<PauseState>) acquires PauseState {
        pause(caller, obj);
    }

    #[test_only]
    public fun test_unpause(caller: &signer, obj: Object<PauseState>) acquires PauseState {
        unpause(caller, obj);
    }

    #[test_only]
    public fun test_update_pauser(caller: &signer, obj: Object<PauseState>, new_pauser: address) acquires PauseState {
        update_pauser(caller, obj, new_pauser)
    }

    #[test_only]
    public fun set_paused_for_testing(obj_address: address, paused: bool) acquires PauseState {
        borrow_global_mut<PauseState>(obj_address).paused = paused;
    }

    #[test_only]
    public fun set_pauser_for_testing(obj_address: address, pauser: address) acquires PauseState {
        borrow_global_mut<PauseState>(obj_address).pauser = pauser;
    }
}
