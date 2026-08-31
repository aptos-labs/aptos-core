// A partial opaque callee can both return and abort without exposing a usable
// abort predicate. WP must retain a source-level partial boundary for the
// caller after discarding the uninformative `aborts_if true` candidate.
module 0x42::partial_abort_fallback {
    fun maybe_abort() {}

    spec maybe_abort {
        pragma opaque;
        pragma verify = false;
        pragma aborts_if_is_partial;
        pragma inference = none;
    }

    fun caller() {
        maybe_abort()
    }
    spec caller() {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] ensures_of<maybe_abort>();
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `partial_abort_fallback::caller` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition did not survive a memory-havocking loop
   ┌─ tests/inference/partial_abort_fallback.move:14:5
   │
14 │ ╭     fun caller() {
15 │ │         maybe_abort()
16 │ │     }
   │ ╰─────^

Verification: exiting with condition generation errors
error: this function has no specification but is referenced by a behavioral predicate
  ┌─ partial_abort_fallback.enriched.move:5:5
  │
5 │     fun maybe_abort() {}
  │     ^^^^^^^^^^^^^^^^^^^^
*/
