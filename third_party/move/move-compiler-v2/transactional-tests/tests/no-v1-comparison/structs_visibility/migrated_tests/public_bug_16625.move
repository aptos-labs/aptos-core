//# publish
module 0xCAFE::m {
    public struct S has copy, drop {
        f: || (u8),
    }
}

//# publish
module 0xCAFE::test_m {
    use 0xCAFE::m::S;

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

//# run 0xCAFE::test_m::main
