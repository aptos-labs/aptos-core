module 0xc0ffee::cool {
    public fun beans(): u64 {
        42
    }
}

module 0xc0ffee::m {
    inline fun foo(f: | | u64): u64 {
        use 0xc0ffee::cool::beans;
        beans();  // discharge unused use warning
        f()
    }

    public fun bar(): u64 {
        foo(| | beans())
    }
}
