//# publish
module 0xc0ffee::m {

    struct S has copy, drop {
        a: u64,
        b: u64,
    }

    fun test(s: S): u64 {
        let p = s;
        let q = p;
        let ref = &mut p.a;
        *ref = 0;
        q.a
    }

    public fun main() {
        assert!(test(S{a: 1, b: 2}) == 1, 5);
    }
}

//# run 0xc0ffee::m::main
