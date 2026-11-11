// A closure `|y| t(w, y)` capturing every function-typed parameter of `t` has known effects:
// what `t` writes includes the write of R2 its call to the native `bump` reaches, which `t`'s
// usage summary omits. The memory havoced across a call it is passed to must include it.
// Expected: each false claim fails as a post-condition, and `ok_r4` verifies. The deferral shape
// is in fun_param_frame_captured_deferred.

// `call_g` declares a frame covering R2: the havoc may not fall below it.
module 0x42::fun_param_frame_captured_declared {
    struct R2 has key { v: u64 }

    native fun bump(y: u64);
    spec bump {
        pragma opaque;
        modifies global<R2>(@0x42);
        aborts_if false;
    }

    fun pure_fn(z: u64) { let _ = z; }

    fun t(g: |u64| has copy + drop, x: u64) {
        g(x);
        bump(x);
    }

    fun call_g(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        modifies_of<f>(_x: u64) R2[@0x42];
        aborts_if false;
    }

    // False: the closure writes R2.
    fun client(x: u64) {
        let w: |u64| has copy + drop = |z| pure_fn(z);
        call_g(|y| t(w, y), x);
    }
    spec client {
        pragma aborts_if_is_partial;
        ensures global<R2>(@0x42) == old(global<R2>(@0x42));
    }
}

// The captured `w` writes R3 under `t`'s declared frame for `g`, and `call_g`'s frame covers
// both writes.
module 0x42::fun_param_frame_captured_declared_framed {
    struct R2 has key { v: u64 }
    struct R3 has key { v: u64 }
    struct R4 has key { v: u64 }

    native fun bump(y: u64);
    spec bump {
        pragma opaque;
        modifies global<R2>(@0x42);
        aborts_if false;
    }

    native fun bump3(y: u64);
    spec bump3 {
        pragma opaque;
        modifies global<R3>(@0x42);
        aborts_if false;
    }

    fun t(g: |u64| has copy + drop, x: u64) {
        g(x);
        bump(x);
    }
    spec t {
        modifies_of<g>(_x: u64) R3[@0x42];
    }

    fun call_g(f: |u64| has copy + drop, x: u64) { f(x) }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        modifies_of<f>(_x: u64) R2[@0x42], R3[@0x42];
        aborts_if false;
    }

    fun ok_r4(x: u64) {
        let w: |u64| has copy + drop = |z| bump3(z);
        call_g(|y| t(w, y), x);
    }
    spec ok_r4 {
        pragma aborts_if_is_partial;
        ensures global<R4>(@0x42) == old(global<R4>(@0x42));
    }

    // False: `w` writes R3.
    fun r3(x: u64) {
        let w: |u64| has copy + drop = |z| bump3(z);
        call_g(|y| t(w, y), x);
    }
    spec r3 {
        pragma aborts_if_is_partial;
        ensures global<R3>(@0x42) == old(global<R3>(@0x42));
    }
}
