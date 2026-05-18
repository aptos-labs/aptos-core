module 0x42::m {
    // E-prefixed constants are error codes included in the on-chain error map.
    // They should not trigger unused warnings even when not referenced by name.
    const ENOT_FOUND: u64 = 1;
    const EINVALID_ARGUMENT: u64 = 2;
    const EALREADY_EXISTS: u64 = 3;

    // Non-E-prefixed constants that ARE used - no warning
    const USED_CONST: u64 = 42;

    // Non-E-prefixed constant that is NOT used - should warn
    const UNUSED_THRESHOLD: u64 = 100;

    public fun do_something(): u64 {
        USED_CONST
    }
}
