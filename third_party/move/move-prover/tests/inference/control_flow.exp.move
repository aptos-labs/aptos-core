// Test spec inference for control flow (if-then-else)
module 0x42::control_flow {

    // Both branches return same parameter - should infer: ensures result == x
    fun if_same_return(x: u64, cond: bool): u64 {
        if (cond) {
            x
        } else {
            x
        }
    }
    spec if_same_return(x: u64, cond: bool): u64 {
        pragma opaque = true;
        ensures [inferred] result == x;
        aborts_if [inferred] false;
    }


    // Both branches assign and return same parameter - should infer: ensures result == x
    fun if_same_assign(x: u64, cond: bool): u64 {
        let result;
        if (cond) {
            result = x;
        } else {
            result = x;
        };
        result
    }
    spec if_same_assign(x: u64, cond: bool): u64 {
        pragma opaque = true;
        ensures [inferred] result == x;
        aborts_if [inferred] false;
    }


    // Different parameters in branches - uses path-conditional ensures
    fun if_different_return(x: u64, y: u64, cond: bool): u64 {
        if (cond) {
            x
        } else {
            y
        }
    }
    spec if_different_return(x: u64, y: u64, cond: bool): u64 {
        pragma opaque = true;
        ensures [inferred] cond ==> result == x;
        ensures [inferred] !cond ==> result == y;
        aborts_if [inferred] false;
    }


    // One branch returns parameter, other computes - uses path-conditional ensures
    fun if_mixed_return(x: u64, cond: bool): u64 {
        if (cond) {
            x
        } else {
            x + 1
        }
    }
    spec if_mixed_return(x: u64, cond: bool): u64 {
        pragma opaque = true;
        ensures [inferred] cond ==> result == x;
        ensures [inferred] !cond ==> result == x + 1;
        aborts_if [inferred] !cond && x == MAX_U64;
    }


    // Chained comparison with short-circuit - uses path-conditional ensures
    fun test_chain_cmp(x: u64, y: u64, z: u64): bool {
        x < y && y < z
    }
    spec test_chain_cmp(x: u64, y: u64, z: u64): bool {
        pragma opaque = true;
        ensures [inferred] x < y ==> result == (y < z);
        ensures [inferred] x >= y ==> result == false;
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
