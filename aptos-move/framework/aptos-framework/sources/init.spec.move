spec aptos_framework::init {
    // native function
    spec get_caller_address_and_module_id(): (address, ModuleId) {
        // The result is derived by the VM from the calling stack frame, which cannot be
        // modeled in the specification language; treated as uninterpreted.
        pragma opaque;
    }

    spec internal_maybe_initialize(only_once: bool): Option<signer> {
        // The caller address is the opaque, VM-derived result of
        // `get_caller_address_and_module_id` (see above), so the object-ownership abort
        // condition cannot be modeled here; abort conditions are treated as partial.
        pragma aborts_if_is_partial;
    }

    spec fun spec_modules(addr: address): OrderedMap<ModuleId, ModuleState> {
        global<InitializationState>(addr).modules
    }

    spec ensure_module_state(addr: address, module_id: ModuleId) {
        aborts_if false;
        modifies global<InitializationState>(addr);
        ensures exists<InitializationState>(addr);
        ensures old(exists<InitializationState>(addr))
            && ordered_map::spec_contains_key(old(spec_modules(addr)), module_id)
            ==> spec_modules(addr) == old(spec_modules(addr));
        ensures !(old(exists<InitializationState>(addr))
            && ordered_map::spec_contains_key(old(spec_modules(addr)), module_id))
            ==> ordered_map::spec_get(spec_modules(addr), module_id)
                == ModuleState { only_once: option::spec_none(), deploy_owner: option::spec_none() };
        ensures ordered_map::spec_contains_key(spec_modules(addr), module_id);
        ensures old(exists<InitializationState>(addr)) ==> spec_modules(addr)
            == ordered_map::spec_set(old(spec_modules(addr)), module_id,
                ordered_map::spec_get(spec_modules(addr), module_id));
    }

    spec check_and_set_initialized(addr: address, module_id: ModuleId, only_once: bool): bool {
        let pre_exists = exists<InitializationState>(addr)
            && ordered_map::spec_contains_key(spec_modules(addr), module_id);
        let pre_state = ordered_map::spec_get(spec_modules(addr), module_id);
        let initialized = pre_exists && option::is_some(pre_state.only_once);
        aborts_if false;
        modifies global<InitializationState>(addr);
        ensures result == initialized;
        let post state = ordered_map::spec_get(spec_modules(addr), module_id);
        ensures ordered_map::spec_contains_key(spec_modules(addr), module_id);
        ensures state.only_once == (if (initialized) { pre_state.only_once } else { option::spec_some(only_once) });
        ensures state.deploy_owner == (if (pre_exists) { pre_state.deploy_owner } else { option::spec_none() });
    }

    spec fun spec_module_id(name: vector<u8>): ModuleId {
        ModuleId { hash: from_bcs::deserialize<u128>(hash::sha3_256(name)[0..16]) }
    }

    spec record_deploy_owner(owner: &signer, addr: address, module_name: vector<u8>) {
        let digest = hash::sha3_256(module_name);
        let module_id = spec_module_id(module_name);
        let pre_exists = exists<InitializationState>(addr)
            && ordered_map::spec_contains_key(spec_modules(addr), module_id);
        let pre_state = ordered_map::spec_get(spec_modules(addr), module_id);
        aborts_if !exists<ObjectCore>(addr);
        aborts_if global<ObjectCore>(addr).owner != signer::address_of(owner);
        aborts_if len(digest) < 16;
        aborts_if !from_bcs::deserializable<u128>(digest[0..16]);
        modifies global<InitializationState>(addr);
        let post state = ordered_map::spec_get(spec_modules(addr), module_id);
        ensures ordered_map::spec_contains_key(spec_modules(addr), module_id);
        ensures option::is_some(state.deploy_owner);
        ensures !exists<ObjectCore>(option::borrow(state.deploy_owner));
        ensures state.only_once == (if (pre_exists) { pre_state.only_once } else { option::spec_none() });
        ensures old(exists<InitializationState>(addr)) ==> spec_modules(addr)
            == ordered_map::spec_set(old(spec_modules(addr)), module_id, state);
    }

    spec assert_may_self_initialize(addr: address, module_id: ModuleId) {
        // Partial: the root owner comparison goes through the ownership chain walk.
        pragma aborts_if_is_partial;
        let recorded = if (exists<InitializationState>(addr)
            && ordered_map::spec_contains_key(spec_modules(addr), module_id)) {
            ordered_map::spec_get(spec_modules(addr), module_id).deploy_owner
        } else {
            option::spec_none()
        };
        aborts_if option::is_none(recorded) && exists<ObjectCore>(addr);
        aborts_if option::is_some(recorded) && !exists<ObjectCore>(addr);
        aborts_if option::is_some(recorded) && exists<ObjectCore>(option::borrow(recorded));
    }

    spec reset_initialized(addr: address, module_names: vector<vector<u8>>) {
        // Partial: the module id of each name is derived inside the loop.
        pragma aborts_if_is_partial;
        modifies global<InitializationState>(addr);
        ensures exists<InitializationState>(addr) == old(exists<InitializationState>(addr));
    }
}
