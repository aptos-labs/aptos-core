// exclude_for: lean
// The Lean backend prints `(forall oid: u64: oid >= 0) && oid >= target` without the
// parentheses, which extends the inner quantifier over the rest and changes the clause; it
// then leaves the true specification below unproved.

// Companion of schema_arg_capture.move: a binder nested in the scope of a renamed binder,
// with the same name. It binds its own uses and is not a use of the outer one.
module 0x42::schema_arg_capture_shadow {

    spec schema Shadow {
        target: u64;
        ensures forall oid: u64: (forall oid: u64: oid >= 0) && oid >= target;
    }

    fun shadow(): u64 { 7 }
    // True: `forall oid: u64: (forall oid: u64: oid >= 0) && oid >= 0`.
    spec shadow { let oid = 0; include Shadow { target: oid }; }

    fun shadow_incorrect(): u64 { 7 }
    // False at 0. Only the outer binder has `target` in its scope outside the inner one.
    spec shadow_incorrect { let oid = 7; include Shadow { target: oid }; }
}
