// Regression test for bug: Operation::Vector with no args emitted `vector[]`
// (no type annotation) instead of `vector<T>[]`.
//
// When WP back-propagates through an assignment `v = vector<T>[]`, it substitutes
// that empty vector literal into the postcondition.  Without the fix in the
// Operation::Vector sourcifier arm, the literal is printed as `vector[]` which
// fails type inference during verification (the compiler cannot determine the
// element type of the untyped literal).
//
// flag: -T=20
module 0x42::vector_typing {

    // Simplest case: WP generates `ensures result == vector<bool>[]`.
    // Without the fix: emits `vector[]` → type-inference error at verification.
    // With the fix: emits `vector<bool>[]` → compiles and verifies.
    fun empty_bool_vec(): vector<bool> {
        let v: vector<bool> = vector[];
        v
    }

    // Same pattern with a different element type to confirm type parameter is
    // taken from the node type (not hard-coded).
    fun empty_u64_vec(): vector<u64> {
        let v: vector<u64> = vector[];
        v
    }
}
