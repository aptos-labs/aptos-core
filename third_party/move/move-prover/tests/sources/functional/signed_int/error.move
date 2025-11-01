module 0x42::InvalidOr {
    struct Int_test {
        x : i64,
    }

    fun test(foo: &mut Int_test) {
        foo.x = foo.x + 1;
    }

    spec Int_test {
        invariant x >= (x ^ (1 as i64));
        invariant x >= (x | (1 as i64));
        invariant x >= (x & (1 as i64));
        invariant x >= (x << (1 as i64));
        invariant x >= (x >> (1 as i64));
    }
}
