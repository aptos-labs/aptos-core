module 0x42::m {
    struct Resource has key {
        value: u64
    }

    // Private entry function - should NOT warn (callable from transactions)
    entry fun init_resource(account: &signer) {
        move_to(account, Resource { value: 0 });
    }

    // Private entry function that calls another private function - should NOT warn
    entry fun increment(account: &signer) acquires Resource {
        let addr = std::signer::address_of(account);
        let resource = borrow_global_mut<Resource>(addr);
        add_one(&mut resource.value);
    }

    // Private helper function used by entry function - should NOT warn
    fun add_one(value: &mut u64) {
        *value = *value + 1;
    }

    // Private function not used by anyone - SHOULD warn
    fun never_called() {
    }

    // Public entry function - should NOT warn
    public entry fun public_init(account: &signer) {
        move_to(account, Resource { value: 100 });
    }

    // Private entry function with complex logic - should NOT warn
    entry fun complex_entry(account: &signer, x: u64) acquires Resource {
        if (x > 0) {
            init_resource(account);
        } else {
            let addr = std::signer::address_of(account);
            if (exists<Resource>(addr)) {
                let Resource { value: _ } = move_from<Resource>(addr);
            };
        };
    }

    // Private function used only in spec - SHOULD warn (specs don't count as usage)
    fun spec_only_function(): u64 {
        42
    }

    spec module {
        fun spec_helper(): u64 {
            spec_only_function()
        }
    }
}
