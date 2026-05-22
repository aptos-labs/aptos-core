// @checks=strict
// Empty peer module used as a friend so that `friend` test helpers
// in `tests_friend` are syntactically legal.
module 0x42::caller {}

module 0x42::tests {
    // ─── flagged: `#[test]` ────────────────────────────────────────────

    // Flagged: visibility only on `#[test]`
    #[test]
    public fun test_public() {}

    // Flagged: `entry` only on `#[test]`
    #[test]
    entry fun test_entry() {}

    // Flagged: visibility and `entry` on `#[test]`
    #[test]
    public entry fun test_public_entry() {}

    // Flagged: `package` visibility on `#[test]`
    #[test]
    package fun test_public_package() {}

    // ─── flagged: `#[test_only]` ───────────────────────────────────────

    // Flagged: `entry` alone on a `#[test_only]` helper
    #[test_only]
    entry fun test_only_entry() {}

    // Flagged: `entry` combined with `public` on a `#[test_only]` helper
    #[test_only]
    public entry fun test_only_public_entry() {}

    // ─── not flagged ────────────────────────────────────────────────────

    // OK: `#[test_only] public fun` is the cross-package helper pattern
    #[test_only]
    public fun test_only_public_helper() {}

    // OK: `#[test_only] package fun` is the same-package pattern
    #[test_only]
    package fun test_only_package_helper() {}

    // OK: private `#[test]`
    #[test]
    fun test_private() {}

    // OK: private `#[test_only]`
    #[test_only]
    fun test_only_private() {}

    // OK: `#[test]` fire suppressed via `lint::skip`
    #[test]
    #[lint::skip(improper_test_function_modifiers)]
    public entry fun suppressed_test() {}

    // OK: `#[test_only]` fire suppressed via `lint::skip`
    #[test_only]
    #[lint::skip(improper_test_function_modifiers)]
    entry fun suppressed_test_only() {}
}

// Separate module because `friend` and `package` cannot
// coexist within the same module.
module 0x42::tests_friend {
    friend 0x42::caller;

    // OK: `#[test_only] friend fun` is the cross-module pattern
    #[test_only]
    friend fun test_only_friend_helper() {}
}

// `#[test_only]` module: each function inherits `#[test_only]` from the
// module, so `entry` is flagged but visibility modifiers are allowed.
#[test_only]
module 0x42::test_only_module {
    // OK: `public` inside a `#[test_only]` module
    public fun module_level_public() {}

    // OK: `package`
    package fun module_level_package() {}

    // OK: private
    fun module_level_private() {}

    // Flagged: `entry` inside a `#[test_only]` module
    entry fun module_level_entry() {}
}
