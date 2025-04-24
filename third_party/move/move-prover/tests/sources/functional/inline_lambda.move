module 0x42::Test {
    // TODO(#16256): add it back when we support mutable parameter/free variables for lambda expression in spec
    // use std::vector;
    // use std::signer;

    // public inline fun inline_1<X: drop>(v: &mut vector<X>, predicate: |&mut X| (bool, bool)) {
    //     let i = 0;
    //     let z = 3;
    //     z = z + 1;
    //     let (x, _) = predicate(vector::borrow_mut(v, i));
    //     if (x) {
    //         vector::swap_remove(v, i);
    //     } else {
    //         i = i + 1;
    //     };
    // }

    // public inline fun inline_2<X: drop>(v: &mut vector<X>, predicate: |&mut X|) {
    //     predicate(vector::borrow_mut(v, 0));
    // }

    // struct R has key, drop {
    //     v: u64
    // }

    // public fun test_inline_1(s: &signer) acquires R {
    //     let v = vector[1u64, 2, 3];
    //     let z;
    //     let r = R {
    //         v: 2
    //     };
    //     move_to<R>(s, r);
    //     let b = borrow_global_mut<R>(signer::address_of(s));
    //     inline_1(&mut v, |e| { *e = 2; b.v = 3; z = 2; (*e > 1, z == 2) }
    //     spec {
    //         requires e == 1;
    //         ensures e == 2;
    //         ensures result_1 == (e > 1);
    //         ensures result_2;
    //         ensures z == 2;
    //         ensures b.v == 3;
    //     }
    //     );
    // }

    // public fun test_inline_fail_1() {
    //     let v = vector[1u64, 2, 3];
    //     inline_1(&mut v, |e| { *e = 2; (*e > 1, false) }
    //     spec {
    //         requires e > 1; // this does not verify
    //     }
    //     );
    // }

    // public fun test_inline_fail_2() {
    //     let v = vector[1u64, 2, 3];
    //     inline_1(&mut v, |e| { *e = 2; (*e > 1, false) }
    //     spec {
    //         ensures e != 2; // this does not verify
    //     }
    //     );
    // }


    // public fun test_inline_2() {
    //     let v = vector[1u64, 2, 3];
    //     inline_2(&mut v, |e| { *e = 2; }
    //     spec {
    //         ensures e == 2;
    //     }
    //     );
    // }

}
