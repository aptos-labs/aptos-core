module 0xc0ffee::m {
    struct P has copy, drop { q: Q }
    struct Q has copy, drop { r: u64 }

    // The double-inline case from #17615
    inline fun derive(p: &mut P): &mut u64 {
        &mut p.q.r
    }

    inline fun update(p: P): P {
        *derive(&mut p) = 20;
        p
    }

    public fun main() {
        assert!(update(P { q: Q { r: 0 } }).q.r == 20, 0);
    }
}
