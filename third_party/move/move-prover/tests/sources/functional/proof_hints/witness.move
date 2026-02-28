// flag: --language-version=2.4
module 0x42::witness_hints {

    // ============================================================
    // Positive: simple witness with a constant.
    fun test_witness_constant(): u64 {
        42
    }
    spec test_witness_constant {
        ensures exists y: u64: y == 42;

        proof {
            witness y = 42 in exists y: u64: y == 42;
        }
    }

    // ============================================================
    // Positive: witness using an expression.
    fun test_witness_expr(): u64 {
        10
    }
    spec test_witness_expr {
        ensures exists y: u64: y > 5 && y <= 10;

        proof {
            witness y = 10 in exists y: u64: y > 5 && y <= 10;
        }
    }

    // ============================================================
    // Error: witness on a non-existential (forall) expression.
    fun test_witness_not_exists(x: u64): u64 {
        x
    }
    spec test_witness_not_exists {
        ensures result == x;

        proof {
            witness y = x in forall y: u64: y == y;
        }
    }

    // ============================================================
    // Error: witness variable not found in quantifier.
    fun test_witness_wrong_var(x: u64): u64 {
        x
    }
    spec test_witness_wrong_var {
        ensures exists y: u64: y == x;

        proof {
            witness z = x in exists y: u64: y == x;
        }
    }
}
