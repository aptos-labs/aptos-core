// A reader of R captured for `hof`'s `f`, which declares `reads_of<f> R`: the closure over `hof`
// reads R through it, so it is not within the empty frame of the parameter it is passed to.
// Expected: ERR at each call to `apply2` in both modules (the false claims are not verified).
module 0x42::fun_param_frame_captured_reader {
    struct R has key { v: u64 }

    fun read_r(a: address, x: u64): u64 acquires R { R[a].v + x }
    spec read_r {
        aborts_if !exists<R>(a) || R[a].v + x > MAX_U64;
        ensures result == R[a].v + x;
    }

    fun bump(a: address) acquires R { R[a].v = 0; }

    fun hof(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec hof {
        reads_of<f> R;
    }

    fun apply2(g: |u64| u64 has copy + drop, x: u64): u64 { g(x) }
    spec apply2 {
        pragma opaque;
        aborts_if aborts_of<g>(x);
        ensures result == result_of<g>(x);
    }

    // False: `bump` changes what `g` reads.
    fun test(a: address): bool acquires R {
        let f: |u64| u64 has copy + drop = |x| read_r(a, x);
        let g: |u64| u64 has copy + drop = |y| hof(f, y);
        let r1 = apply2(g, 1);
        bump(a);
        let r2 = apply2(g, 1);
        r1 == r2
    }
    spec test {
        pragma aborts_if_is_partial;
        ensures result;
    }
}

// `apply2` is verified under the empty frame of `g`, which it relies on.
module 0x42::fun_param_frame_captured_reader_body {
    struct R has key { v: u64 }

    fun read_r(a: address, x: u64): u64 acquires R { R[a].v + x }

    fun hof(f: |u64| u64 has copy + drop, x: u64): u64 { f(x) }
    spec hof {
        reads_of<f> R;
    }

    // False for a `g` that reads R.
    fun apply2(g: |u64| u64 has copy + drop, a: address): bool acquires R {
        let r1 = g(1);
        R[a].v = R[a].v + 1;
        let r2 = g(1);
        r1 == r2
    }
    spec apply2 {
        pragma aborts_if_is_partial;
        ensures result;
    }

    fun test(a: address): bool acquires R {
        let f: |u64| u64 has copy + drop = |x| read_r(a, x);
        let g: |u64| u64 has copy + drop = |y| hof(f, y);
        apply2(g, a)
    }
}
