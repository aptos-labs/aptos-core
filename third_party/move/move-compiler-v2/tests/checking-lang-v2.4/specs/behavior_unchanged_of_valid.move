// Tests for valid parsing and typing of the `unchanged_of` behavior predicate.

module 0x42::M {

    struct R has key, drop { v: u64 }

    // Function parameter target: checked like the other behavior predicates.
    fun apply_unchanged(f: |u64|, x: u64) {
        f(x)
    }

    spec apply_unchanged {
        ensures unchanged_of<f>(x);
    }

    // Binary function parameter target.
    fun apply_unchanged_binary(f: |u64, u64|, a: u64, b: u64) {
        f(a, b)
    }

    spec apply_unchanged_binary {
        ensures unchanged_of<f>(a, b);
    }

    // Lambda target of an inline function: substituted by the inliner with
    // the frame condition over the lambda's derived write footprint.
    inline fun apply_one(f: |address|, a: address, other: address) {
        f(a);
        spec {
            assert other != a ==> unchanged_of<f>(other);
        };
    }

    fun caller(a: address, b: address) {
        apply_one(
            |x| {
                let v = R[x].v;
                *&mut R[x] = R { v: v + 1 };
            },
            a,
            b,
        );
    }
}
