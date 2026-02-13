// Test spec inference for function calls and closures using behavioral predicates
module 0x42::function_calls {

    // Helper function with spec for testing behavioral predicates
    fun callee(x: u64): u64 {
        x + 1
    }
    spec callee {
        ensures result == x + 1;
        aborts_if x == 18446744073709551615; // MAX_U64
        ensures [inferred] x != 18446744073709551615 ==> result == x + 1;
        aborts_if [inferred] x > MAX_U64 - 1;
    }

    // Direct function call - should infer using ensures_of/aborts_of
    fun test_call(x: u64): u64 {
        callee(x)
    }
    spec test_call(x: u64): u64 {
        ensures [inferred] result == result_of<callee>(x);
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Chained function calls
    fun test_call_chain(x: u64): u64 {
        callee(callee(x))
    }
    spec test_call_chain(x: u64): u64 {
        ensures [inferred] result == at_3@result_of<callee>(result_of<callee>(x)@at_3);
        aborts_if [inferred] at_3@aborts_of<callee>(result_of<callee>(x)@at_3);
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Function call result stored in variable
    fun test_call_assign(x: u64): u64 {
        let y = callee(x);
        y
    }
    spec test_call_assign(x: u64): u64 {
        ensures [inferred] result == result_of<callee>(x);
        aborts_if [inferred] aborts_of<callee>(x);
    }


    // Higher-order function with closure invocation
    fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply(f: |u64|u64, x: u64): u64 {
        ensures [inferred] result == result_of<f>(x);
        aborts_if [inferred] aborts_of<f>(x);
    }


    // Function that uses apply with callee
    fun test_higher_order(x: u64): u64 {
        apply(|v| callee(v), x)
    }
    spec test_higher_order(x: u64): u64 {
        ensures [inferred] result == result_of<apply>(|arg0| callee(arg0), x);
        aborts_if [inferred] aborts_of<apply>(|arg0| callee(arg0), x);
    }

}
/*
Verification: Succeeded.
*/
