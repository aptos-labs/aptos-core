module 0x42::test{

    struct S has key, drop{
        g: G,
    }
    struct G has store, drop{
        u: u64
    }

    public fun f1(s_ref: &S) acquires S {
        let _ = &mut s_ref.g;
    }

    public fun f2() acquires S {
        let s = borrow_global<S>(@0x1);
        &mut s.g;
    }

    public fun f3(): u64 acquires S {
        let g = G {u:2};
        let s = S {g};
        let s_ref = &s;
        let x = &mut s_ref.g;
        x.u
    }

    public fun no_error(): u64 acquires S {
        let g = G {u:2};
        let s = S {g};
        let s_ref = &mut s;
        let x = &mut s_ref.g;
        x.u
    }


    struct A has key, drop{
        b: B,
    }

    struct B has store, drop{
        g: G,
    }

    public fun f4(): u64 acquires S {
        let g = G {u:2};
        let b = B {g};
        let a = A {b};
        let a_ref = &mut a;
        let b_ref = &a_ref.b;
        let x = &mut b_ref.g;
        x.u
    }

    public fun no_error_2(): u64 acquires S {
        let g = G {u:2};
        let b = B {g};
        let a = A {b};
        let a_ref = &mut a;
        let b_ref = &mut a_ref.b;
        let x = &mut b_ref.g;
        x.u
    }

}
