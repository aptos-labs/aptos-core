// Public constants are part of the module's API and should not trigger
// unused warnings, even if not referenced within the module itself.
module 0x42::m {
    // public const: should NOT warn (external callers may use it)
    public const MAX: u64 = 100;

    // private const with no users: SHOULD warn
    const INTERNAL: u64 = 42;

    public fun api(): u64 {
        MAX
    }
}
