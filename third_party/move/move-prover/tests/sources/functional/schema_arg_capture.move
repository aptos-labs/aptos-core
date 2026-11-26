// An argument of a schema include denotes the value of the expression in the including
// specification block. A binder inside the schema that happens to have the name of a
// variable in the argument must not capture it.
//
// Every function named `*_incorrect` has a false specification and must be reported. The
// other functions have true specifications and must verify; each is the true twin of a
// false one, so the false ones are not rejected for an unrelated reason.
module 0x42::schema_arg_capture {

    spec schema Ens { target: u64; ensures forall oid: u64: oid >= target; }
    spec schema EnsWhere {
        target: u64;
        ensures forall oid: u64 where oid > target: oid > 7;
    }
    spec schema Abt { target: u64; aborts_if exists oid: u64: oid != target; }
    spec schema Req { target: u64; requires forall oid: u64: oid >= target; }

    // ---------------- the binder is a quantifier variable ----------------

    fun let_same_name_incorrect(): u64 { 7 }
    // `forall oid: u64: oid >= 7` is false at 0.
    spec let_same_name_incorrect { let oid = 7; include Ens { target: oid }; }

    fun let_other_name_incorrect(): u64 { 7 }
    spec let_other_name_incorrect { let t = 7; include Ens { target: t }; }

    fun param_same_name_incorrect(oid: u64): u64 { oid }
    spec param_same_name_incorrect { requires oid == 7; include Ens { target: oid }; }

    fun let_zero(): u64 { 7 }
    // True twin: `forall oid: u64: oid >= 0`.
    spec let_zero { let oid = 0; include Ens { target: oid }; }

    fun let_same_name_premise(): u64 { 7 }
    // True: every u64 above 7 is above 7. With the argument captured the premise reads
    // `oid > oid`, the clause is vacuous, and this cannot tell the two readings apart; it
    // is here as the control for the next one.
    spec let_same_name_premise { let oid = 7; include EnsWhere { target: oid }; }

    fun let_same_name_premise_incorrect(): u64 { 7 }
    // False: 7 is above 6 and not above 7. With the argument captured the premise reads
    // `oid > oid` and the false clause verifies.
    spec let_same_name_premise_incorrect { let oid = 6; include EnsWhere { target: oid }; }

    fun aborts_same_name_incorrect(): u64 { 7 }
    // The function never aborts, and `exists oid: u64: oid != 7` is true.
    spec aborts_same_name_incorrect { let oid = 7; include Abt { target: oid }; }

    fun aborts_other_name_incorrect(): u64 { 7 }
    spec aborts_other_name_incorrect { let t = 7; include Abt { target: t }; }

    fun callee(): u64 { 7 }
    // Not provable at the call below: `forall oid: u64: oid >= 7` is false.
    spec callee { let oid = 7; include Req { target: oid }; }

    fun caller_incorrect(): u64 { callee() }

    // ---------------- the argument is a compound expression ----------------

    fun compound_incorrect(): u64 { 7 }
    // `forall oid: u64: oid >= 3 + 4` is false at 0.
    spec compound_incorrect { let oid = 3; include Ens { target: oid + 4 }; }

    // ---------------- nested binders and several ranges ----------------

    spec schema Nested {
        target: u64;
        ensures forall oid: u64: (exists oid: u64: oid == target) && oid >= target;
    }

    fun nested_incorrect(): u64 { 7 }
    // The inner statement `exists oid: u64: oid == 7` is true; the outer `oid >= 7` is
    // false at 0.
    spec nested_incorrect { let oid = 7; include Nested { target: oid }; }

    spec schema TwoRanges {
        target: u64;
        ensures forall oid: u64, other: u64: oid + other >= target;
    }

    fun two_ranges_incorrect(): u64 { 7 }
    spec two_ranges_incorrect { let other = 7; include TwoRanges { target: other }; }

    // ---------------- the binder is a let inside an expression ----------------

    spec schema InnerLet {
        target: u64;
        ensures { let oid = 1; oid + target } == 2;
    }

    fun inner_let_incorrect(): u64 { 7 }
    // `1 + 7 == 2` is false. Captured, it reads `1 + 1 == 2`.
    spec inner_let_incorrect { let oid = 7; include InnerLet { target: oid }; }

    fun inner_let(): u64 { 7 }
    // True twin: `1 + 1 == 2`.
    spec inner_let { let oid = 1; include InnerLet { target: oid }; }

    // ---------------- the binder is in a schema included by the schema ----------------

    spec schema Outer { outer_target: u64; include Ens { target: outer_target }; }

    fun through_two_schemas_incorrect(): u64 { 7 }
    spec through_two_schemas_incorrect { let oid = 7; include Outer { outer_target: oid }; }

    // ---------------- a binder which captures nothing keeps its name ----------------

    spec schema Unrelated {
        target: u64;
        unused: u64;
        ensures forall oid: u64: oid >= target;
    }

    fun unrelated_arg_incorrect(): u64 { 7 }
    // False at 0. The argument which mentions `oid` is not used under the binder `oid`, so
    // that binder is not renamed; the Lean baseline shows the clause with its name kept.
    spec unrelated_arg_incorrect { let oid = 3; include Unrelated { target: 7, unused: oid }; }

    // ---------------- a post-state let ----------------

    fun post_let_incorrect(): u64 { 7 }
    spec post_let_incorrect { let post oid = result; include Ens { target: oid }; }
}
