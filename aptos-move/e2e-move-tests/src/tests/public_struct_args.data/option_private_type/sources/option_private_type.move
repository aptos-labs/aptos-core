/// Test module verifying Option<Hero> is valid as a transaction argument when Hero is private.
/// Option is a whitelisted struct, so its type argument is not validated at compile time or
/// at VM validation time — the caller can only ever pass None (since Hero has no constructor),
/// and None is accepted.
module 0xcafe::option_private_type {
    use std::option::Option;
    use std::signer;

    /// Private struct — not public, has no pack function.
    struct Hero has copy, drop {
        health: u64,
    }

    struct TestResult has key {
        called: bool,
    }

    public entry fun initialize(sender: &signer) {
        move_to(sender, TestResult { called: false });
    }

    /// Entry function taking Option<Hero>. Only None can be passed since Hero is private.
    public entry fun accept_option_hero(
        sender: &signer,
        opt: Option<Hero>,
    ) acquires TestResult {
        assert!(std::option::is_none(&opt), 0);
        borrow_global_mut<TestResult>(signer::address_of(sender)).called = true;
    }

    #[view]
    public fun was_called(addr: address): bool acquires TestResult {
        borrow_global<TestResult>(addr).called
    }
}
