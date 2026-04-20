// Test that behavioral predicates correctly handle functions returning immutable
// references (&T). Bug 7d: `behavioral_output_types` previously only stripped
// `&mut T` from result types, leaving `&T` mapped to `$Mutation(int)` in Boogie
// instead of `int`, causing a Boogie type mismatch.
//
// `peek` takes &mut Data (→ first check in try_as_pure_spec_call fails, making
// it a behavior predicate) and returns &u64 (borrows a field). The Boogie
// result function `$bp_result_of'peek'(...)` must be declared as returning
// `int`, not `$Mutation(int)`.
module 0x42::ref_result_pred {
    struct Data has copy, drop { value: u64 }

    // &mut param + &T return: always a behavior predicate.
    // result == self.value: spec auto-derefs the &u64 return.
    fun peek(self: &mut Data): &u64 {
        &self.value
    }
    spec peek {
        pragma opaque = true;
        pragma inference = none;
        aborts_if false;
        ensures result == self.value;
        ensures self.value == old(self.value);
    }

    // Dereferences the &u64 result from peek.
    // WP: result = *$t0, $t0 = result_of<peek>(self) → result = result_of<peek>(self).
    // The Boogie axiom for peek's result_of must use int, not $Mutation(int).
    fun caller(self: &mut Data): u64 {
        *peek(self)
    }
}
