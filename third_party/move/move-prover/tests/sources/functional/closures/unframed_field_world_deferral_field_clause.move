// A predicate over an unframed field ranges over the world memory, so a contract stating it is not
// memory-free and does not leave its closure parameter's frame to the call site: otherwise the
// call site would havoc what the closure writes and then assume the stability clause across it.
module 0x42::store {
    struct R has key { v: u64 }
    public fun set(v: u64) acquires R { R[@0x42].v = v }
    public fun check(x: u64) acquires R { assert!(R[@0x42].v != x, 1) }
}

module 0x42::dwf {
    use 0x42::store;
    struct H has drop, copy { cb: |u64| has copy + drop }

    public fun apply(f: |u64|, h: H, x: u64) { f(x) }
    spec apply {
        pragma opaque;
        pragma verify = false;
        aborts_if aborts_of<f>(x);
        ensures aborts_of<h.cb>(x) == old(aborts_of<h.cb>(x));
    }

    // False: `h.cb` may be `store::check`, which aborts once the closure has stored `x`.
    public fun later_call_no_abort(h: H, x: u64) {
        apply(|y| store::set(y), h, x);
        (h.cb)(x)
    }
    spec later_call_no_abort {
        requires exists<store::R>(@0x42);
        requires !aborts_of<h.cb>(x);
        aborts_if false;
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
