module 0x42::m {
    use std::vector;

    public fun test1() {
        let i = 0;
        let v = vector[1, 2, 3];
        let _v = &mut v;
        vector::swap_remove(_v, 0);
        while ({
            spec {
                invariant v == vector[1, 2, 3]; // this should not verify
            };
            (i < vector::length(_v))
        }) {
            if (*vector::borrow(_v, i) > 1) {
                vector::swap_remove(_v, i);
            } else {
                i = i + 1;
            };
        }
    }

    struct R has key, drop {
        v: u64
    }

    public fun test2() acquires R {
        // let i = 0;
        let v = 3;
        let _v = &mut v;
        *_v = 4;
        let x = borrow_global_mut<R>(@0x1);
        *x = R {
            v: 3
        };
        spec {
            assert v == 4; // this should verify
            assert global<R>(@0x1).v == 3; // this should verify
        };
        *x = R {
            v: 6
        };
        *_v = 5;
    }

   public fun test3() {
        let i = 0;
        let v = vector[1, 2, 3];
        let _v = &mut v;
        while ({
            (i < vector::length(_v))
        }) {
            spec {
                 assert v == vector[1, 2, 3]; // this should not verify
            };
            if ( *vector::borrow(_v, i) > 1) {
                vector::swap_remove(_v, i);
            } else {
                i = i + 1;
            };
            spec {
                    assert v == vector[1, 2, 3]; // this should not verify
            };
        }
    }

    public fun test4() {
        let i = 0;
        let v = vector[1, 2, 3];
        let _v = &mut v;
        if (*vector::borrow(_v, i) <= 1) {
                vector::swap_remove(_v, i);
            } ;
        spec {
            assert v == vector[1, 2, 3]; // this should not verify
        };
        if (*vector::borrow(_v, i) <= 1) {
                vector::swap_remove(_v, i);
        } ;
    }

}
