module 0xc0ffee::m {
    #[persistent]
    inline fun forty_two(): u64 {
        42
    }

    #[module_lock]
    inline fun fifty_five(): u64 {
        55
    }
}
