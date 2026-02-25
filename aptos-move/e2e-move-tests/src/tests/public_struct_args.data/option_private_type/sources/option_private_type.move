/// Test module verifying Option<T> behavior when T has restricted constructability:
/// - Option<Hero>: Hero is private (no pack function). Only None is accepted.
/// - Option<NoCopyData>: NoCopyData is public but lacks copy. Only None is accepted;
///   Some(NoCopyData{...}) fails at construction time because the copy check rejects it.
/// Option is whitelisted, so its type argument bypasses the copy-ability check at validation
/// time. The copy check is enforced at construction time when the inner value must be built.
module 0xcafe::option_private_type {
    use std::option::Option;

    /// Private struct — not public, has no pack function.
    struct Hero has copy, drop {
        health: u64,
    }

    /// Public struct without copy ability — a pack function is generated (public visibility
    /// is sufficient), but only None can be passed when used as Option's type argument,
    /// since the copy check rejects construction of the inner value for Some.
    public struct NoCopyData has drop {
        value: u64,
    }

    /// Entry function taking Option<Hero>. Only None can be passed since Hero is private.
    public entry fun accept_option_hero(
        _sender: &signer,
        opt: Option<Hero>,
    ) {
        assert!(std::option::is_none(&opt), 0);
    }

    // Entry function taking Option<NoCopyData>. Only None can be passed; Some(NoCopyData{...})
    // fails at construction time because NoCopyData lacks copy ability.
    public entry fun accept_option_nocopy(
        _sender: &signer,
        _opt: Option<NoCopyData>,
    ) {}

    /// View function taking Option<NoCopyData>. Same rules as the entry function:
    /// None succeeds; Some(NoCopyData{...}) fails at construction time.
    #[view]
    public fun is_option_nocopy_none(opt: Option<NoCopyData>): bool {
        std::option::is_none(&opt)
    }
}
