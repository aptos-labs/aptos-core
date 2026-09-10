// RUN: publish --print(stackless)
module 0x42::copy_prop_enum_bypass {
    enum Holder has copy, drop { V { n: u64 } }

    fun make(seed: u64): Holder {
        Holder::V { n: seed }
    }

    // Takes ownership and mutates the payload in place.
    fun set_payload(h: Holder): u64 {
        let owned = h;
        owned.n = 99;
        owned.n
    }

    // Enum values are heap-boxed: `d` is a deep copy of `a`, and the callee's
    // in-place write must not be visible through `a`.
    fun bypass(seed: u64): u64 {
        let a = make(seed);
        let d = copy a;
        let pin = d.n; // field borrow pins `d` in a home slot
        let got = set_payload(d);
        a.n * 100 + got + pin
    }
}

// RUN: execute 0x42::copy_prop_enum_bypass::bypass --args 1
// CHECK: results: 200
