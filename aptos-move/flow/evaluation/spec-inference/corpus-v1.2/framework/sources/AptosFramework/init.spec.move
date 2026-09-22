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
}
