//# publish
module 0xCAFE::m {
    struct S has copy, drop {
        f: || (u8),
    }

    fun foo(x: S) {
        *(&mut (
            (x.f)()
        )) = 1;
    }

    public fun main() {
        let s = S { f: || 42 };
        foo(s);
    }
}

//# run 0xCAFE::m::main
