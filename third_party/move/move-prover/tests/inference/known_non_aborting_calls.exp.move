module 0x42::known_non_aborting_calls {
    use std::string;

    fun constant(): u64 {
        7
    }
    spec constant(): u64 {
        pragma opaque = true;
        ensures [inferred] result == 7;
        aborts_if [inferred] false;
    }


    fun calls_inferred_no_abort(): u64 {
        constant()
    }
    spec calls_inferred_no_abort(): u64 {
        pragma opaque = true;
        ensures [inferred] result == constant();
        aborts_if [inferred] false;
    }


    fun constructs_empty_string(): bool {
        string::utf8(b"").length() == 0
    }
    spec constructs_empty_string(): bool {
        use 0x1::string;
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] result == (string::length(string::utf8(vector<u8>[])) == 0);
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `known_non_aborting_calls::constructs_empty_string` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition did not survive a memory-havocking loop
   ┌─ tests/inference/known_non_aborting_calls.move:12:5
   │
12 │ ╭     fun constructs_empty_string(): bool {
13 │ │         string::utf8(b"").length() == 0
14 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
