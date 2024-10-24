module 0xdecafbad::m {
    fun foo(f: ||) {
        f();
    }

    public fun one() {
        foo(|| {});
    }

    fun bar(f:||u64): u64 {
        f()
    }

    public fun two(x:u64): u64 {
        bar(||x)
    }
}
