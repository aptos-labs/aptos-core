/// Aborts and implicit failures, with the locations they are reported at.
module fixtures::failures {
    use fixtures::helper;

    #[test]
    #[expected_failure(abort_code = 7, location = Self)]
    fun abort_here() {
        abort 7
    }

    #[test]
    #[expected_failure(abort_code = 1, location = fixtures::helper)]
    fun abort_in_callee() {
        helper::fail(1);
    }

    #[test]
    #[expected_failure(abort_code = 2)]
    fun abort_code_only() {
        helper::fail(2);
    }

    #[test]
    #[expected_failure(arithmetic_error, location = Self)]
    fun overflow() {
        let _ = helper::max() + 1;
    }

    #[test]
    #[expected_failure(arithmetic_error, location = fixtures::helper)]
    fun division_by_zero_in_callee() {
        helper::divide(1, 0);
    }

    #[test]
    #[expected_failure(vector_error, minor_status = 1, location = Self)]
    fun index_out_of_bounds() {
        let v = vector[1, 2];
        let _ = v[v.length() + 3];
    }

    #[test]
    #[expected_failure]
    fun any_failure() {
        helper::divide(1, 0);
    }
}

module fixtures::helper {
    public fun fail(code: u64) {
        abort code
    }

    public fun max(): u64 {
        18446744073709551615
    }

    public fun divide(a: u64, b: u64): u64 {
        a / b
    }
}
