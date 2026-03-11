// Tests that #[immutable] on a private constant is rejected.
module 0x42::M {
    // Error: private constant with #[immutable] has no accessor to pin.
    #[immutable]
    const PRIV: u64 = 42;

    public fun use_it(): u64 { PRIV }
}
