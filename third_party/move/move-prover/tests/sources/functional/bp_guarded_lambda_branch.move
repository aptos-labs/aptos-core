// exclude_for: cvc5
// Regression: a lambda whose body calls a `&mut` unit-return function under a
// branch gets a WP-inferred spec with a guarded `ensures_of` anchor
// (`c ==> ensures_of<f>(inputs)`). Anchor canonicalization must look through
// the guard: previously the input-only anchor leaked to Boogie ("wrong number
// of arguments to $bp_ensures_of...") and an unguarded canonical over-claimed
// the callee's ensures on paths that never call it.
module 0x42::bp_guarded_lambda_branch {

    fun dec(s: &mut u64, d: u64) {
        *s = *s - d;
    }

    spec dec {
        aborts_if s < d;
        ensures s == old(s) - d;
    }

    fun apply(f: |&mut u64| has drop, x: &mut u64) {
        f(x)
    }

    public fun caller(x: &mut u64, c: bool, d: u64) {
        apply(|s| if (c) { dec(s, d) }, x)
    }

    // Nested branches produce nested guards (`c ==> (e ==> ensures_of<dec>)`);
    // all implication layers must be unwrapped, not just the outermost.
    public fun caller_nested(x: &mut u64, c: bool, e: bool, d: u64) {
        apply(|s| if (c) { if (e) { dec(s, d) } }, x)
    }
}
