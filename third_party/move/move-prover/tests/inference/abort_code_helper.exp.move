// A helper used only to construct an abort code cannot contribute a normal
// postcondition or make an unconditional abort continuation incomplete.
module 0x42::abort_code_helper {
    use std::error;

    fun divide(a: u64, b: u64): u64 {
        assert!(b != 0, error::invalid_argument(4));
        a / b
    }
    spec divide(a: u64, b: u64): u64 {
        pragma opaque = true;
        ensures [inferred] b != 0 ==> result == a / b;
        aborts_if [inferred] b == 0;
    }


    fun always_aborts() {
        abort error::invalid_argument(4)
    }

    // Shape of the VS-shares-002 failure, including the inlined helper and cast.
    inline fun mul_div(a: u64, b: u64, c: u64): u64 {
        assert!(c != 0, error::invalid_argument(4));
        (((a as u128) * (b as u128) / (c as u128)) as u64)
    }

    fun shares(shares: u64, nav: u64, total: u64): u64 {
        assert!(total >= shares, 1);
        mul_div(shares, nav, total)
    }
    spec shares(shares: u64, nav: u64, total: u64): u64 {
        pragma opaque = true;
        ensures [inferred] total >= shares && total != 0 ==> result == (((shares as u128) * (nav as u128) / (total as u128)) as u64);
        aborts_if [inferred] total < shares;
        aborts_if [inferred] total == 0;
    }


    // An incomplete callee on a returning path must still be reported.
    fun unknown(x: u64): u64 { x + 1 }
    spec unknown {
        pragma opaque;
        pragma verify = false;
        pragma inference = none;
        pragma aborts_if_is_partial;
        ensures result == x + 1;
    }

    fun returning_call(x: u64): u64 { unknown(x) }
    spec returning_call(x: u64): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == unknown(x);
    }


    fun guarded_call(x: u64, call: bool): u64 {
        if (call) { unknown(x) } else { 0 }
    }
    spec guarded_call(x: u64, call: bool): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == (if (call) unknown(x) else 0);
    }


    // There is no normal return, but the abort continuation is conditional:
    // an unknown callee before it must not be mistaken for an abort-code helper.
    fun conditional_abort(x: u64) {
        let value = unknown(x);
        loop {
            spec { invariant true; };
            if (value != 0) { break };
        };
        abort 1
    }
    spec conditional_abort(x: u64) {
        pragma opaque = true, aborts_if_is_partial = true;
        aborts_if [inferred] unknown(x) != 0;
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `abort_code_helper::returning_call` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::abort_code_helper::unknown` has no trusted complete abort summary
   ┌─ tests/inference/abort_code_helper.move:36:5
   │
36 │     fun returning_call(x: u64): u64 { unknown(x) }
   │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

warning: WP could not characterize the aborts of `abort_code_helper::guarded_call` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::abort_code_helper::unknown` has no trusted complete abort summary
   ┌─ tests/inference/abort_code_helper.move:38:5
   │
38 │ ╭     fun guarded_call(x: u64, call: bool): u64 {
39 │ │         if (call) { unknown(x) } else { 0 }
40 │ │     }
   │ ╰─────^

warning: WP could not characterize the aborts of `abort_code_helper::conditional_abort` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::abort_code_helper::unknown` has no trusted complete abort summary
   ┌─ tests/inference/abort_code_helper.move:44:5
   │
44 │ ╭     fun conditional_abort(x: u64) {
45 │ │         let value = unknown(x);
46 │ │         loop {
47 │ │             spec { invariant true; };
   · │
50 │ │         abort 1
51 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
