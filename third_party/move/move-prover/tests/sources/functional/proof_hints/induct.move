// flag: --language-version=2.4
module 0x42::proof_hint_induct {

    // ============================================================
    // Induction on an integer parameter.

    fun identity(n: u64): u64 {
        n
    }
    spec identity {
        ensures result == n;

        proof {
            induct on n;
        }
    }

    // ============================================================
    // Error: induct on non-integer type.

    fun test_induct_bad_type(s: bool): bool {
        s
    }
    spec test_induct_bad_type {
        ensures result == s;

        proof {
            induct on s;
        }
    }

    // ============================================================
    // Error: induct on non-parameter.

    fun test_induct_non_param(x: u64): u64 {
        x
    }
    spec test_induct_non_param {
        ensures result == x;

        proof {
            induct on z;
        }
    }
}
