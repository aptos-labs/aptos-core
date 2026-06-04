// Test-planner row semantics are ignored when test code is disabled.
address 0x1 {
module M {
    #[test(addr = @0x1), test(addr = @0x2)]
    fun multiple_tests_in_one_bracket(addr: signer) {
        let _ = addr;
    }

    #[test(addr = @0x1), deprecated]
    fun unrelated_row_sibling(addr: signer) {
        let _ = addr;
    }

    #[test(addr = @0x1), test_only]
    fun test_only_row_sibling(addr: signer) {
        let _ = addr;
    }

    #[test(real = @0x1, typo = @0x2)]
    fun unknown_argument(real: signer) {
        let _ = real;
    }

    #[test]
    fun empty_parameterized_row(addr: signer) {
        let _ = addr;
    }

    #[test(addr = @0x1), expected_failure]
    #[expected_failure]
    fun row_local_and_top_level_failure(addr: signer) {
        let _ = addr;
    }

    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    #[expected_failure(abort_code = 5, location = 0x1::M)]
    fun parameterized_top_level_failure(addr: signer) {
        let _ = addr;
    }

    #[test(addr = @0x1)]
    #[test(typo = @0x2)]
    fun invalid_row_rejects_valid_sibling(addr: signer) {
        let _ = addr;
    }
}
}
