//# publish
module 0x42::test {
    struct Impotent {}

    fun test() {
        let x = Impotent {};
        let y = &x;
        y;   // must error that x is dropped
    }

    struct S {
        f: u64,
        g: T
    }

    struct T {
        h: u64
    }

    fun read_val(x: S): u64 {
        x.g.h
    }
}
