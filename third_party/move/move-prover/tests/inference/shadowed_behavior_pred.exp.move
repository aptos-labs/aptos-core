// Test for bug fix: shadowed behavior-predicate target in user-written spec.
//
// When a callee generates behavior predicates in the WP (e.g. because it takes
// a &mut parameter, so try_as_pure_spec_call returns None), the WP emits
// result_of<callee>(args) / aborts_of<callee>(args). If the caller's spec has
// a user-written let-binding with the same name as the callee, re-parsing the
// inferred condition resolves that name to the let-binding (u64), not the
// function → "behavior predicate target must have function type, found u64".
//
// Fix: print_behavior_target in the sourcifier always emits fully qualified
// names (addr::module::fn) for behavior predicate targets. This ensures the
// name resolves to the function regardless of any local let-binding with the
// same name.
module 0x42::shadowed_behavior_pred {
    struct S has copy, drop { value: u64 }

    // Takes &mut → try_as_pure_spec_call returns None → WP uses behavior
    // predicates (result_of<set_value> / aborts_of<set_value>) for any
    // caller that calls set_value.
    fun set_value(s: &mut S, v: u64) {
        s.value = v;
    }
    spec set_value {
        ensures s.value == v;
        aborts_if false;
        pragma opaque = true;
        ensures [inferred] s == update_field(old(s), value, v);
        aborts_if [inferred] false;
    }

    // User spec has `let set_value = 0u64` — a let-binding whose name shadows
    // the module function `set_value`. The WP generates aborts_of<set_value>(s, v).
    // After the fix, the sourcifier emits the fully qualified name, so the
    // condition becomes aborts_of<0x42::shadowed_behavior_pred::set_value>(s, v)
    // which re-parses correctly and verifies.
    fun do_set(s: &mut S, v: u64) {
        set_value(s, v);
    }
    spec do_set {
        pragma verify = false;
        let set_value = 0u64; // shadows the function 'set_value'
        pragma opaque = true;
        ensures [inferred] ensures_of<0x42::shadowed_behavior_pred::set_value>(s, v, s);
        aborts_if [inferred] aborts_of<0x42::shadowed_behavior_pred::set_value>(s, v);
    }
}
/*
Verification: Succeeded.
*/
