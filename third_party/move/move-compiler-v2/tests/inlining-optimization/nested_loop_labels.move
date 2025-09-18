module 0xc0ffee::m {
    fun outer(x: u64): u64 {
        'a: loop {
            inner(x);
            x = x + 1;
            break 'a;
        };
        x
    }

    fun inner(x: u64): u64 {
        'a: loop {
            x = x + 10;
            break 'a;
        };
        x
    }
}
