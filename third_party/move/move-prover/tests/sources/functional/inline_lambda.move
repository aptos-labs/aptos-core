module 0x42::Test {
    use std::vector;
    use std::signer;

    public inline fun filter_mut<X: drop>(v: &mut vector<X>, predicate: |&mut X| (bool, bool)) {
        let i = 0;
        let z = 3;
        while ({
            spec {
                // TODO: this will cause a no-such-function error as `predicate`
                // is inlined away in the implementation. We are aware of this
                // issue and is working on a fix by bridging more information
                // from inlined functions into the spec.
                // invariant forall k in 0..i: !predicate(v[k]);
                // TODO: complete the set of loop invariants
            };
            (i < vector::length(v))
        }) {
            z = z + 1;
            let (x, _) = predicate(vector::borrow_mut(v, i));
            if (x) {
                vector::swap_remove(v, i);
            } else {
                i = i + 1;
            };
        }
    }

    public inline fun filter<X: drop>(v: &mut vector<X>, predicate: |&X| bool) {
        let i = 0;
        while ({
            spec {
                // TODO: this will cause a no-such-function error as `predicate`
                // is inlined away in the implementation. We are aware of this
                // issue and is working on a fix by bridging more information
                // from inlined functions into the spec.
                // invariant forall k in 0..i: !predicate(v[k]);
                // TODO: complete the set of loop invariants
            };
            (i < vector::length(v))
        }) {
            if (predicate(vector::borrow(v, i))) {
                vector::swap_remove(v, i);
            } else {
                i = i + 1;
            };
        }
    }

    public fun filter2() {
        let i = 0;
        let v = vector[1, 2, 3];
        let _v = &mut v;

        while ({
            // spec {
            //     // TODO: this will cause a no-such-function error as `predicate`
            //     // is inlined away in the implementation. We are aware of this
            //     // issue and is working on a fix by bridging more information
            //     // from inlined functions into the spec.
            //     invariant forall k in 0..i: !predicate(v[k]);
            //     // TODO: complete the set of loop invariants
            // };
            (i < vector::length(_v))
        }) {
            //let x = predicate(vector::borrow(v, i));
            if ( {let x = _v[i] > 1; spec {assert v == vector[1, 2, 3]; }; x}) {
                vector::swap_remove(_v, i);
            } else {
                i = i + 1;
            };
        }
    }

    // public fun test_filter(): vector<u64> {
    //     let v = vector[1u64, 2, 3];
    //     // let z = 4;
    //     spec {
    //         //let x = 1;
    //         //assert x == 1;
    //     };
    //     filter(&mut v, |e| {  *e > 1}
    //     spec {
    //         // ensures e
    //         //requires z >= 1;
    //         //requires v == vector[1, 2, 3];
    //         ensures result == (e > 1);//v == vector[1, 2, 3];
    //         //requires false;
    //         //ensures false;
    //         //let x = 1;
    //         //requires v == vector[1, 2, 3];
    //         //let post x = 3;
    //         //ensures v == vector[1, 2, 3];
    //         // assert false;
    //     }
    //     );
    //     // result = result + 1;
    //     v
    // }
    // spec test_filter {
    //     pragma verify = true;
    //     //requires result == 3;
    //     //let x = 2;
    //     //let post result = 3;
    //     //let result = 3;
    //     //let post x = x + 3;
    //     //ensures x == 5;
    //     // ensures result == vector[];
    //     // TODO: turn-on the verification once inlining on spec side is done
    // }

    // public fun test_filter_3(): u64 {
    //     let v = vector[1u64, 2, 3];
    //     let z;
    //     // let z = 4;
    //     spec {
    //         //let x = 1;
    //         //assert z == 3;
    //     };
    //     z
        //filter_mut(&mut v, |e| { *e = 2;  z = 2;  (*e > 1, z == 2) }
        // spec {
        //     // ensures e
        //     //requires z >= 1;
        //     //requires v == vector[1, 2, 3];
        //     //requires z == 3;
        //     //requires z == 3;
        //     ensures e == 2;
        //     ensures result_1 == (e > 1);//v == vector[1, 2, 3];
        //     ensures result_2;//v == vector[1, 2, 3];
        //     ensures z == 2;
        //     //requires false;
        //     //ensures false;
        //     //let x = 1;
        //     //requires v == vector[1, 2, 3];
        //     //let post x = 3;
        //     //ensures v == vector[1, 2, 3];
        //     // assert false;
        // }
        //);
        // result = result + 1;
        //v
    //}

    struct R has key, drop {
        v: u64
    }

    public fun test_filter_2(s: &signer): vector<u64> acquires R {
        let v = vector[1u64, 2, 3];
        let z;

        let r = R {
            v: 2
        };
        move_to<R>(s, r);
        let b = borrow_global_mut<R>(signer::address_of(s));
        // let z = 4;
        spec {
            //let x = 1;
            // assert z == 3;
        };
        filter_mut(&mut v, |e| { *e = 2; b.v = 3; z = 2; (*e > 1, z == 2) }
        spec {
            // ensures e
            //requires z >= 1;
            //requires v == vector[1, 2, 3];
            //requires z == 3;
            //requires z == 3;
            ensures e == 2;
            ensures result_1 == (e > 1);//v == vector[1, 2, 3];
            ensures result_2;//v == vector[1, 2, 3];
            ensures z == 2;
            ensures b.v == 3;
            // ensures global<R>(signer::address_of(s)).v == 2;
            //requires false;
            //ensures false;
            //let x = 1;
            //requires v == vector[1, 2, 3];
            //let post x = 3;
            //ensures v == vector[1, 2, 3];
            // assert false;
        }
        );
        // result = result + 1;
        v
    }
    spec test_filter_2 {
        pragma verify = true;
        //requires result == 3;
        //let x = 2;
        //let post result = 3;
        //let result = 3;
        //let post x = x + 3;
        //ensures x == 5;
        // ensures result == vector[];
        // TODO: turn-on the verification once inlining on spec side is done
    }
}
