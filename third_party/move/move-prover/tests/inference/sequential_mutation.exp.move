// Test for bug fix: nested old() and wrong pre-state in sequential mutations.
//
// When two functions with &mut params are called sequentially, the WP for the
// second call must use the post-state of the first call as its pre-state.
//
// Bug: substitute_multiple_temps_in_state replaced `c_temp` inside `old(c_temp)`
// in the state, turning `old(c_temp)` into `old(result_of<first>(...))`. Then
// substitute_old_param_in_state also replaced `old(c_temp)` inside the substitution
// value, producing a double-application: result_of<f>(result_of<f>(c, a), a)
// instead of result_of<f>(c, a) for the second call's pre-state. The spurious
// extra old() wrapper also created nested old(old(...)) which Move syntax forbids.
//
// Fix: wp_function_call skips already-captured &mut params in all_subs so
//   substitute_multiple_temps_in_state never touches old(param) patterns.
//   Only substitute_old_param_in_state handles them, replacing old(c) directly
//   with result_of<f>(old(c), a) — no extra old() wrapper, no double application.
//
module 0x42::sequential_mutation {
    struct Counter has copy, drop {
        value: u64,
    }

    fun add_amount(c: &mut Counter, amount: u64) {
        c.value = c.value + amount;
    }
    spec add_amount {
        ensures c.value == old(c.value) + amount;
        aborts_if false;
        pragma opaque = true;
        ensures [inferred] c == update_field(old(c), value, old(c).value + amount);
        aborts_if [inferred] c.value + amount > MAX_U64;
    }

    // Two sequential mutations. The WP for the second add_amount call must use
    // result_of<add_amount>(old(c), a) as the pre-state (post-state of first
    // call). Before the fix, the pre-state was incorrectly computed as
    // result_of<add_amount>(result_of<add_amount>(c, a), a) — applying
    // add_amount twice with `a` instead of once. After the fix the pre-state is
    // correct, and ensures conditions for both calls verify cleanly.
    fun add_twice(c: &mut Counter, a: u64, b: u64) {
        add_amount(c, a);
        add_amount(c, b);
    }
    spec add_twice(c: &mut Counter, a: u64, b: u64) {
        pragma opaque = true;
        ensures [inferred] ..S1 |~ ensures_of<add_amount>(old(c), a);
        ensures [inferred] S1.. |~ ensures_of<add_amount>(c, b, c);
        aborts_if [inferred] S1 |~ aborts_of<add_amount>(c, b);
        aborts_if [inferred] aborts_of<add_amount>(c, a);
    }

}
/*
Verification: Succeeded.
*/
