// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Companion to `spec_fun_old_param.move`: exercises spec functions that take a
// `&mut T` parameter and use `old(p)` under a *labeled* state range
// (`..S |~`, `S.. |~`). Covers two encoder gaps `wrap_mut_ref_spec_fun_inputs`
// exposed and that are fixed in this PR:
//
//   1. `find_value_state_for_label` (boogie-backend/src/spec_translator.rs)
//      must recognize labeled `SpecFunction` calls with a `&mut` parameter, so
//      `exists S in *` emits `S_val: T` as the state-domain bound variable.
//      Without this, Boogie sees `exists  :: (...)` (empty binder list) and
//      rejects the file with a parse error.
//
//   2. `translate_spec_fun_call` (same file) must substitute the bound
//      `S_val` into the doubled `(Old(arg), arg)` slots whenever `range.pre`
//      or `range.post` corresponds to a value-typed state variable, mirroring
//      the substitution path used by behavioral-predicate translation.
//      Without this, every doubled call would compare function-entry to
//      function-exit regardless of the label range.
//
// Note on test strength: `ensures exists S in *: ...` is encoded as
// `assume (exists S_val: T :: P(S_val))` in the BPL (same as the pre-existing
// `followed_by_mut_ref.move` BP test). The existential-in-ensures therefore
// does not act as a tight verification predicate at the body's exit — that
// is an orthogonal concern in the state-domain quantifier lowering and is
// out of scope here. The correctness signal this test carries is twofold:
// (a) the labeled spec-function call parses cleanly through Boogie, and
// (b) the emitted call references `S_val` in the correct slot per range.pre /
// range.post — both verifiable by inspecting the generated `output.bpl`.

module 0x42::param_old_labeled_repro {
    struct Counter has copy, drop, store { value: u64 }

    /// Returns true when the counter strictly increased between the pre- and
    /// post-state slots of the call (whichever labels the call binds).
    spec fun counter_increased(c: &mut Counter): bool {
        old(c).value < c.value
    }

    fun inc(c: &mut Counter) {
        c.value = c.value + 1;
    }

    fun inc_twice(c: &mut Counter) {
        inc(c);
        inc(c)
    }
    spec inc_twice {
        ensures exists S in *:
            (..S |~ counter_increased(c)) &&  // entry → S
            (S.. |~ counter_increased(c));    // S → exit
    }
}
