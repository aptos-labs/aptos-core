// flag: --language-version=2.4
module 0x42::proof_hint_trigger {

    // ============================================================
    // Trigger hint: add E-matching trigger to a quantifier.

    spec fun is_valid(x: u64): bool;

    fun check_trigger_passthrough(x: u64): u64 {
        x
    }
    spec check_trigger_passthrough {
        ensures forall y: u64: is_valid(y) ==> is_valid(y);

        proof {
            trigger forall y: u64 with {is_valid(y)};
        }
    }

    // ============================================================
    // Error: trigger that doesn't mention all bound variables.

    spec fun marker(x: u64): bool { true }

    fun test_trigger_missing_var(x: u64): u64 {
        x
    }
    spec test_trigger_missing_var {
        ensures forall a: u64, b: u64: marker(a) && marker(b);

        proof {
            trigger forall a: u64, b: u64 with {marker(a)};
        }
    }

    // ============================================================
    // Warning: trigger with only interpreted operations.

    fun test_trigger_interpreted(x: u64): u64 {
        x
    }
    spec test_trigger_interpreted {
        ensures forall y: u64: y + 0 == y;

        proof {
            trigger forall y: u64 with {y + 1};
        }
    }
}
