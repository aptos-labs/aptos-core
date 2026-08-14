// Inferring a spec for a function which conditionally calls a `&mut` unit-return
// function produces a guarded `ensures_of` anchor (`c ==> ensures_of<dec>(..)`).
// Anchor canonicalization must look through the guard: previously the input-only
// anchor leaked to Boogie ("wrong number of arguments to $bp_ensures_of...") and
// an unguarded canonical over-claimed the callee's ensures on paths that never
// call it.
module 0x42::guarded_call {

    fun dec(s: &mut u64, d: u64) {
        *s = *s - d;
    }

    spec dec {
        aborts_if s < d;
        ensures s == old(s) - d;
        pragma opaque = true;
        ensures [inferred] s == old(s) - d;
        aborts_if [inferred] s - d < 0;
    }

    fun caller(s: &mut u64, c: bool, d: u64) {
        if (c) {
            dec(s, d)
        }
    }
    spec caller(s: &mut u64, c: bool, d: u64) {
        pragma opaque = true;
        ensures [inferred] !c ==> s == old(s);
        ensures [inferred] c ==> ensures_of<dec>(s, d, s);
        aborts_if [inferred] c && aborts_of<dec>(s, d);
    }


    // Nested branches produce nested guards (`c ==> (e ==> ensures_of<dec>)`);
    // all implication layers must be unwrapped, not just the outermost.
    fun caller_nested(s: &mut u64, c: bool, e: bool, d: u64) {
        if (c) {
            if (e) {
                dec(s, d)
            }
        }
    }
    spec caller_nested(s: &mut u64, c: bool, e: bool, d: u64) {
        pragma opaque = true;
        ensures [inferred] c && !e ==> s == old(s);
        ensures [inferred] !c ==> s == old(s);
        ensures [inferred] c && e ==> ensures_of<dec>(s, d, s);
        aborts_if [inferred] c && (e && aborts_of<dec>(s, d));
    }

}
/*
Verification: Succeeded.
*/
