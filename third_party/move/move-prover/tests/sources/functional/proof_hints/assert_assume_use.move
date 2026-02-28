// flag: --language-version=2.4
module 0x42::proof_hint_assert_assume_use {

    // ============================================================
    // Assert lemma in proof block.

    fun increment(_addr: address): u64 {
        1
    }
    spec increment {
        ensures result == 1;

        proof {
            assert 1 == 1;
        }
    }

    // ============================================================
    // Assume [trusted] in proof block.

    fun complex_op(x: u64): u64 {
        x * x
    }
    spec complex_op {
        ensures result == x * x;

        proof {
            assume [trusted] x * x < 18446744073709551615;
        }
    }

    // ============================================================
    // Use hint: instantiate a spec function at specific values.

    spec fun add_commutative(a: u64, b: u64): bool { a + b == b + a }

    fun add_values(a: u64, b: u64): u64 {
        a + b
    }
    spec add_values {
        ensures result == a + b;

        proof {
            use add_commutative(a, b);
        }
    }

    // ============================================================
    // Error: assume without [trusted] annotation.

    fun test_assume_no_trusted(x: u64): u64 {
        x + 1
    }
    spec test_assume_no_trusted {
        proof {
            assume x > 0;
        }
    }

    // ============================================================
    // Error: proof block in struct invariant (not allowed).

    struct MyStruct has key {
        value: u64,
    }
    spec MyStruct {
        proof {
            assert true;
        }
    }
}
