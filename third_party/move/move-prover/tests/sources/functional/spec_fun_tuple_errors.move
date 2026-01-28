// no-boogie-test
// Tests for spec function tuple error cases
module 0x42::TupleErrors {

    // Error: 0-tuple (unit type) - not a real tuple
    spec fun empty(): () { () }

    // Error: 1-tuple - parentheses don't create a tuple, just grouping
    spec fun single(x: u64): (u64) { (x) }

    // Error: tuple too large (9 elements)
    spec fun too_large(a: u64, b: u64, c: u64, d: u64, e: u64,
                       f: u64, g: u64, h: u64, i: u64):
        (u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (a, b, c, d, e, f, g, h, i)
    }

    // Error: tuple way too large (10 elements)
    spec fun way_too_large(a: u64, b: u64, c: u64, d: u64, e: u64,
                           f: u64, g: u64, h: u64, i: u64, j: u64):
        (u64, u64, u64, u64, u64, u64, u64, u64, u64, u64) {
        (a, b, c, d, e, f, g, h, i, j)
    }

    /// Need to use the spec functions, otherwise the monomorphizer will eliminate them.
    fun use_them(): bool { true }
    spec use_them {
        // These are valid - empty() returns unit, single() returns u64
        ensures empty() == ();
        ensures single(42) == 42;
        // These are errors - tuples too large
        ensures too_large(1, 2, 3, 4, 5, 6, 7, 8, 9) == too_large(1, 2, 3, 4, 5, 6, 7, 8, 9);
        ensures way_too_large(1, 2, 3, 4, 5, 6, 7, 8, 9, 10) == way_too_large(1, 2, 3, 4, 5, 6, 7, 8, 9, 10);
    }
}
