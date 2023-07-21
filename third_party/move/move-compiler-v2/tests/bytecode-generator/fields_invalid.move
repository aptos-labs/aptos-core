module 0x42::fields {

    struct S {
        f: u64,
        g: T
    }

    struct T {
        h: u64
    }

    fun write_ref(x: &S) {
        x.g.h = 42;
    }
}
