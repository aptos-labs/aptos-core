module 0x42::m {
    // init_module is a special function that's called automatically by the VM
    // when the module is published. It should NOT be warned as unused.
    fun init_module(account: &signer) {
        // Do some initialization
    }

    // This function should be warned as unused
    fun unused_helper() {
    }
}
