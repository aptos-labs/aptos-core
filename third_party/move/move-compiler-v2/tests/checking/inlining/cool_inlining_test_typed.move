module 0xc0ffee::cool {
    public fun beans(): u64 {
        42
    }
}

module 0xc0ffee::m {
    inline fun foo(f: |u8| u64): u64 {
        use 0xc0ffee::cool::beans;
        beans();  // discharge unused use warning
        f(3)
    }

    public fun bar(): u64 {
        foo(|_x: u8| beans())
    }
}
