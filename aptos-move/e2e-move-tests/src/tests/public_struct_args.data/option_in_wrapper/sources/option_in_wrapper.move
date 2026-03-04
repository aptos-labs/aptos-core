/// Demonstrates that a public copy struct containing Option<T> is a valid transaction argument
/// type even when T is private. The extended checker does not validate T for whitelisted structs
/// like Option, and the VM allows Option<PrivateT> because None is a legitimate value.
/// Only Some(PrivateT) fails — at construction time — since PrivateT has no pack function.
module 0xcafe::option_in_wrapper {
    use std::option::Option;

    /// Private struct — not public, no pack function generated.
    struct Hero has copy, drop {
        health: u64,
    }

    /// Public copy struct with an Option<T> field.
    /// T is non-phantom (stored in the Option), but because Option is whitelisted, the extended
    /// checker does not recurse into T. Wrapper<Hero> is therefore a valid transaction parameter
    /// type: the only constructable value is Wrapper { o: None }.
    public struct Wrapper<T: copy + drop> has copy, drop {
        o: Option<T>,
    }

    /// Entry function: accepts Wrapper<Hero> and asserts the inner Option is None.
    /// With None, the full path succeeds: compile → extended check → VM validation →
    /// construction (no pack function needed for None) → execution.
    public entry fun check_none(w: Wrapper<Hero>) {
        assert!(std::option::is_none(&w.o), 0);
    }

    // View function: accepts Wrapper<Hero> and returns whether the inner Option is None.
    // Same validation/construction rules as the entry function above.
    #[view]
    public fun check_none_view(w: Wrapper<Hero>): bool {
        std::option::is_none(&w.o)
    }
}
