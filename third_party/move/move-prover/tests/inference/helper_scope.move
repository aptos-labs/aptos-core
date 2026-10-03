// flag: --verify-only=helper_scope::caller
// flag: --infer-unspecified-helpers
// Inference scoped to one function also infers its callees which have no
// specification, before the caller; other functions stay untouched.
// inference-reject-mutation: bump(s); => bump(s); bump(s);
module 0x42::helper_scope {
    struct S has drop {
        a: u64,
        b: u64,
    }

    fun bump(s: &mut S) {
        s.b = s.b + 1;
    }

    fun double(x: u64): u64 {
        x * 2
    }

    fun specified(x: u64): u64 {
        x
    }
    spec specified {
        ensures result == x;
        aborts_if false;
    }

    fun caller(s: &mut S, x: u64): u64 {
        bump(s);
        double(specified(x)) + 1
    }

    fun unrelated(x: u64): u64 {
        double(x)
    }
}
