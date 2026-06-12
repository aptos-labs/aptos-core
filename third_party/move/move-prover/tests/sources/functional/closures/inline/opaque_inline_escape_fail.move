// Tests that a closure capturing references cannot outlive a retained
// inline-opaque call: its captured locations are only modeled at that call
// site, so the callee must not be able to leak the function value back to the
// caller, neither through its result (directly or wrapped in a struct) nor
// through a `&mut` parameter. Global storage is excluded already, since
// reference captures lack the `store` ability.
module 0x42::opaque_inline_escape_fail {

    inline fun id_fun(f: |u64|): |u64| {
        f
    }
    spec id_fun {
        pragma opaque;
        ensures result == f;
    }

    /// Test: escape through the result.
    fun test_escape_via_result(): u64 {
        let x = 0;
        let g = id_fun(|i| x = x + i spec { ensures x == old(x) + i; }); // error: callee may leak function values
        g(5);
        x
    }

    inline fun stash(f: |u64| has drop, slot: &mut |u64| has drop) {
        *slot = f
    }
    spec stash {
        pragma opaque;
        ensures slot == f;
    }

    /// Test: escape through a `&mut` parameter.
    fun test_escape_via_mut_param(dummy: |u64| has drop) {
        let x = 0;
        let s = dummy;
        stash(|i| x = x + i spec { ensures x == old(x) + i; }, &mut s); // error: callee may leak function values
        s(1);
    }

    struct Holder has drop {
        f: |u64| has drop,
    }

    inline fun wrap(f: |u64| has drop): Holder {
        Holder { f }
    }
    spec wrap {
        pragma opaque;
    }

    /// Test: escape through a struct-wrapped result.
    fun test_escape_via_struct_result(): u64 {
        let x = 0;
        let _h = wrap(|i| x = x + i spec { ensures x == old(x) + i; }); // error: callee may leak function values
        x
    }

    struct Wrap<T> has drop {
        v: T,
    }

    inline fun wrap_pair(f: |u64| has drop): (Wrap<u64>, Wrap<|u64| has drop>) {
        (Wrap { v: 1 }, Wrap { v: f })
    }
    spec wrap_pair {
        pragma opaque;
    }

    /// Test: escape through a later instantiation of the same generic wrapper
    /// (each instantiation must be inspected separately).
    fun test_escape_via_generic_wrapper(): u64 {
        let x = 0;
        let (_a, _b) = wrap_pair(|i| x = x + i spec { ensures x == old(x) + i; }); // error: callee may leak function values
        x
    }
}
