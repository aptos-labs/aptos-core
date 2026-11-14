// A specification turned into an evaluator (its function is named by `ensures_of`) that applies
// a predicate to a value read from a framed field. A value read from a field may be of unknown
// origin, so the predicate ranges over every memory, which this evaluator receives and forwards:
// the claim verifies.
module 0x42::fg {
    struct R has key { v: u64 }
    struct Cb has drop, copy { f: |u64|u64 has copy + drop }
    spec Cb { reads_of<f> R; }

    fun run(c: Cb, x: u64): u64 { (c.f)(x) }
    spec run {
        pragma opaque;
        pragma aborts_if_is_partial;
        ensures result == result_of<c.f>(x);
    }

    fun client(c: Cb, x: u64): u64 { run(c, x) }
    spec client {
        pragma aborts_if_is_partial;
        ensures ensures_of<run>(c, x, result);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
