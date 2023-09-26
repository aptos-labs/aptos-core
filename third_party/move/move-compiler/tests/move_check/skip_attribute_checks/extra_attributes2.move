// tests non-abort related execution failures with errors in attributes
module 0x1::n {}
module 0x1::m {
    #[test]
    #[expected_failure(arithmetic_error, location=Self)]
    fun t5() { }

    #[test]
    #[expected_failure(abort_code=3, test, location=Self)]
    fun t6() { }

    #[test]
    #[expected_failure(vector_error, test_only, location=Self)]
    fun t7() { }

    #[test_only]
    #[expected_failure(bytecode_instruction, location=Self)]
    fun t8() { }

    #[test]
    #[expected_failure(verify_only)]
    fun t9() { }
}
