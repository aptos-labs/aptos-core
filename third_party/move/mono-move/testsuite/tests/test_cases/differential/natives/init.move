// Differential test for the init::get_caller_address_and_module_id native.

// RUN: publish
module 0x1::init {
    struct ModuleId has copy, drop, store {
        hash: u128
    }

    native fun get_caller_address_and_module_id(): (address, ModuleId);

    // Indirection so the native's caller is this module, not the entry frame.
    fun call_native(): (address, ModuleId) {
        get_caller_address_and_module_id()
    }

    // Caller module is 0x1::init: returns (0x1, hash("init")).
    public fun caller_addr_and_hash(): (address, u128) {
        let (addr, id) = call_native();
        (addr, id.hash)
    }

    // Native called directly from the entry frame: its caller has no module,
    // so the native aborts.
    public fun caller_is_entry(): (address, u128) {
        let (addr, id) = get_caller_address_and_module_id();
        (addr, id.hash)
    }
}

// RUN: execute 0x1::init::caller_addr_and_hash
// CHECK: results: 0x1, 294358983490175456809003430205152366618

// RUN: execute 0x1::init::caller_is_entry
// CHECK-SUBSTR: aborted: code 65537
