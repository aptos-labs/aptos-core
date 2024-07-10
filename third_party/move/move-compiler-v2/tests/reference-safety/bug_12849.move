module 0x42::m {

    struct S has key, drop {
        g: G
    }
    struct G has copy, drop, store {
        id: u64,
    }
    fun foo(x: &S): &G {
        &x.g
    }

    fun foo_mut(_x: &mut S, _y:u64) {
    }

    fun id(g: &G):u64 {
        g.id
    }

    fun test2(
        s: S,
    ) {
        let x =   &mut s;
        let f = foo(x);
        foo_mut(x, id(f));
    }
}
