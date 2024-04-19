module 0x42::tuple_invalid {

    struct S {
        f: u64,
    }

    fun tuple(x: u64): (u64, S) {
        (x, S{f: x + 1})
    }

    fun use_tuple1(x: u64): u64 {
        let x = tuple(x);
        1
    }
}
