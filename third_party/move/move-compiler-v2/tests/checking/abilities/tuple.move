module 0x42::tuple {

    struct S {
        f: u64,
    }

    fun tuple(x: u64): (u64, S) {
        (x, S{f: x + 1})
    }

    fun use_tuple(x: u64): u64 {
        let (x, S{f: y}) = tuple(x);
        x + y
    }
}
