module 0x42::Test {
    use std::vector;

    inline fun apply(v: u64, predicate: |u64| bool): bool {
        spec {
            assert v != 42;
            assert predicate(v);
        };
        predicate(v)
    }

    public fun test_apply(a1: u64, a2: u64) {
        let r1 = apply(0, |v| v >= 0);
        spec {
            assert r1;
        };

        let r2 = apply(0, |v| v != a1 + a2);
        spec {
            assert !r2;
        };
    }

    inline fun inline_1(x: u64, f: |u64|bool, g:|u64|bool, e:|u64|bool) : bool {
        let y = f(x);
        let z = g(x);
        let w = e(x);
        spec {
            assert y == (x > 2);
            assert y == f(x);
            assert z == g(x);
            assert w == e(x); // e is translated into an uninterpreted spec fun
        };
        y
    }

    fun call_inline_1(y: u64) {
        let z = 3 + y;
        inline_1(y, |x| x > 2, |x| x > z, |x| { while(z < y) {let _x = x;}; x > 5 } spec {
            ensures result == (x > 5);
            ensures result != !(x > 5);
        });
    }

    inline fun inline_2(x: u64, e:|u64|bool): bool {
        if (x > 0) {
            let w = e(x);
            spec {
                assert x > 0;
                assert w == e(x); // e is translated into an uninterpreted spec fun
            };
            true
        } else {
            false
        }
    }

    fun call_inline_2(y: u64) {
        let z = 3 + y;
        inline_2(y, |x| { while(z < y) {let _x = x;}; x > 5 } spec {
            requires x > 0;
            ensures result == (x > 5);
            ensures result != !(x > 5);
        });
    }

    fun call_inline_2_aborts_if(y: u64) {
        inline_2(y, |x| { if (x == 0) { abort 1; }; x > 5  } spec {
            aborts_if !(x > 0);
            ensures result == (x > 5);
            ensures result != !(x > 5);
        });
    }

    fun call_inline_fail_1(y: u64) {
        let z = 3 + y;
        inline_1(y, |x| x > 2, |x| x > z, |x| { while(z < y) {let _x = x;}; x > 5 } spec {
            aborts_if x > 0; // This does not verify
            ensures result == (x > 5);
            ensures result != !(x > 5);
        });
    }

    fun call_inline_fail_2(y: u64) {
        let z = 3 + y;
        inline_1(y, |x| x > 2, |x| x > z, |x| { while(z < y) {let _x = x;}; x > 5 } spec
        {
            aborts_if false; // This verifies
            requires x > 0; // This does not verify at the call site
            requires z > 5; // This does not verify at the call site
            invariant x >= 0; // This verifies
            ensures result == (x == 5); //  This does not verify
            ensures result ==> !(x == 5); //  This verifies
        });
    }

    inline fun find<Element>(s: &vector<Element>, f: |&Element|bool): (bool, u64) {
        let find = false;
        let found_index = 0;
        let i = 0;
        let len = vector::length(s);
        while ({
            spec {
               invariant i <= len;
               invariant found_index == 0;
               invariant forall j in 0..i: !f(s[j]);
            };
            i < len
            }) {
            if (f(vector::borrow(s, i))) {
                find = true;
                found_index = i;
                break
            };
            i = i + 1;
        };
        (find, found_index)
    }

    fun call_find() {
        let s = vector[1, 2, 3];
        let (_a, _b) = find(&s, |x| *x > 1);
    }
}
