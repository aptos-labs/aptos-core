//# publish --print-bytecode
module 0xc0ffee::m {
    struct Foo has key {
        x: u64
    }

    fun foo(p: u64, x: &u64): u64 {
        p + *x
    }

    fun bar(_q: u64, _y: u64) {
    }

    public fun test(addr: address, p: u64, q: u64) acquires Foo {
        let x = &borrow_global<Foo>(addr).x;
        let y = foo(p, x);
        bar(q, y);
    }

}
