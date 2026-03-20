/// Test module demonstrating Container<T> enum behavior for two cases:
///
/// Case 1: T = Hero (private, has copy)
///   Container declares copy → passes declared-copy check.
///   - Empty  → succeeds (no inner value to construct)
///   - Value  → fails   (Hero is private, no pack function)
///
/// Case 2: T = NoCopyData (public, no copy)
///   Container declares copy → passes declared-copy check.
///   - Empty  → succeeds (no inner NoCopyData to construct)
///   - Value  → fails   (NoCopyData has no declared copy, rejected at field construction)
///
/// The copy check uses the struct definition's *declared* ability, not the instantiated
/// type's ability. This mirrors Option<T>: None/Empty always succeeds; Some/Value fails
/// only when the inner type cannot be constructed.
module 0xcafe::negative_phantom_option {
    use std::option::Option;

    /// Private struct - not public, has no pack function
    struct Hero has copy, drop {
        health: u64,
        level: u64,
    }

    /// Public struct without copy ability.
    public struct NoCopyData has drop {
        value: u64,
    }

    /// Public struct WITH copy ability wrapping a generic field.
    /// CopyData<T> declares copy, but the instantiation only has copy when T does.
    public struct CopyData<T> has copy, drop {
        data: T,
    }

    /// Public enum with NON-phantom type parameter.
    public enum Container<T> has copy, drop {
        Value { data: T },
        Empty,
    }

    /// Succeeds with Empty; fails with Value{Hero{...}} (Hero is private).
    public entry fun test_container_hero(_sender: &signer, _container: Container<Hero>) {
    }

    /// Empty succeeds (no inner value to construct); Value fails (NoCopyData has no declared copy).
    public entry fun test_container_nocopy(_sender: &signer, _container: Container<NoCopyData>) {
    }

    // View function with the same rules: Empty succeeds, Value fails.
    #[view]
    public fun check_container_nocopy(_container: Container<NoCopyData>): bool {
        true
    }

    /// Option<CopyData<NoCopyData>>: CopyData declares copy but NoCopyData does not.
    /// None succeeds; Some(CopyData{NoCopyData{...}}) fails when NoCopyData is constructed.
    public entry fun test_option_copy_wrapper_nocopy(
        _sender: &signer,
        _opt: Option<CopyData<NoCopyData>>,
    ) {}

    /// Container<CopyData<NoCopyData>>: Container and CopyData both declare copy.
    /// Empty succeeds; Value{CopyData{NoCopyData{...}}} fails when NoCopyData is constructed.
    public entry fun test_container_copy_wrapper_nocopy(
        _sender: &signer,
        _container: Container<CopyData<NoCopyData>>,
    ) {}

    /// Three levels of nesting: Option<CopyData<CopyData<NoCopyData>>>.
    /// All outer wrappers (Option, CopyData, CopyData) declare copy; only the innermost
    /// NoCopyData lacks it.
    /// - None  → succeeds (no value constructed at any level)
    /// - Some  → fails when NoCopyData is reached (no declared copy)
    public entry fun test_option_triple_nested_nocopy(
        _sender: &signer,
        _opt: Option<CopyData<CopyData<NoCopyData>>>,
    ) {}

    /// View function equivalent of test_option_triple_nested_nocopy.
    #[view]
    public fun check_option_triple_nested_nocopy(
        _opt: Option<CopyData<CopyData<NoCopyData>>>,
    ): bool {
        true
    }
}
