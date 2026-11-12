// A continuation stored in an unframed field may read any memory, so `result_of` over it is
// tied to the state the predicate names. `step` writes `R` and then runs the continuation: its
// result is the continuation's at the state after the write, not at entry.
module 0x42::es {
    struct R has key { v: u64 }
    struct Cell has key { f: |u64|u64 has copy + drop + store }

    fun step(x: u64): u64 acquires Cell, R {
        let f = Cell[@0x42].f;
        R[@0x42].v = 7;
        f(x)
    }
    spec step {
        pragma opaque;
        pragma aborts_if_is_partial;
        requires exists<R>(@0x42) && exists<Cell>(@0x42);
        modifies R[@0x42];
        ensures ..S1 |~ update<R>(@0x42, R { v: 7 });
        ensures result == (S1.. |~ result_of<old(Cell[@0x42]).f>(x));
    }

    // False: the continuation runs after the write, not at entry. Rejected.
    fun step_at_entry(x: u64): u64 acquires Cell, R {
        let f = Cell[@0x42].f;
        R[@0x42].v = 7;
        f(x)
    }
    spec step_at_entry {
        pragma aborts_if_is_partial;
        requires exists<R>(@0x42) && exists<Cell>(@0x42);
        ensures result == result_of<old(Cell[@0x42]).f>(x);
    }

    // True, through `step`'s evaluator. Verifies.
    fun client(x: u64): u64 acquires Cell, R { step(x) }
    spec client {
        pragma aborts_if_is_partial;
        requires exists<R>(@0x42) && exists<Cell>(@0x42);
        ensures ensures_of<step>(x, result);
    }

    // False: the client's result is the continuation's after the write. Rejected.
    fun client_at_entry(x: u64): u64 acquires Cell, R { step(x) }
    spec client_at_entry {
        pragma aborts_if_is_partial;
        requires exists<R>(@0x42) && exists<Cell>(@0x42);
        ensures result == result_of<old(Cell[@0x42]).f>(x);
    }
}

// The Lean backend does not implement the memory rules for unframed function values yet.
// exclude_for: lean
