module 0x42::m {
    // init_module is called by VM on publish - not unused
    fun init_module(_account: &signer) {}

    // Entry functions are callable externally - not unused
    public entry fun entry_func() {}

    // Public functions are externally accessible - not unused
    public fun public_func(): u64 { 1 }

    // Friend functions are accessible to friends - not unused
    public(friend) fun friend_func(): u64 { 2 }

    // Private helper called by public function - not unused
    fun private_helper(): u64 { 3 }

    public fun caller(): u64 {
        private_helper()
    }
}
