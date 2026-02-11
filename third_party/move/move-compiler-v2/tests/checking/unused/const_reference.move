module 0x42::m {
    const BASE: u64 = 100;
    const ERR: u64 = BASE + 1;

    public fun use_err(): u64 {
        ERR
    }
}
