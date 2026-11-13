// flag: --verify-only=opaque_conditionless_callee::caller
// An opaque callee whose contract states no condition, like `event::emit`:
// WP names it through `ensures_of`, and the prover must accept that, since
// callers are verified against the contract however little it says. The
// contract leaves its aborts open, so the caller's stay partial.
module 0x42::opaque_conditionless_callee {
    fun record(x: u64) {
        let _ = x;
    }
    spec record {
        pragma opaque;
    }

    fun caller(x: u64): u64 {
        record(x);
        x
    }
    spec caller(x: u64): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == x;
        ensures [inferred] ensures_of<record>(result);
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `opaque_conditionless_callee::caller` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::opaque_conditionless_callee::record` has no trusted complete abort summary
   ┌─ tests/inference/opaque_conditionless_callee.move:14:5
   │
14 │ ╭     fun caller(x: u64): u64 {
15 │ │         record(x);
16 │ │         x
17 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
