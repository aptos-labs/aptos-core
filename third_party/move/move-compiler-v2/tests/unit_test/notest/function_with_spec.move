// #[test_only] functions should be filtered out in non-test mode
address 0x1234 {
module M {
    public fun foo() { }

    #[test]
    public fun bar_test() { bar() }

    #[test_only]
    public fun bar() { }

    // This should not cause an error in either test- nor non-test-mode.
    spec bar {
        aborts_if false;
    }

    // This should always cause an error due to typo.
    spec baz {
        aborts_if false;
    }
}
}
