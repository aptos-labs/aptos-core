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

    // Different parameters in branches - uses path-conditional ensures
    fun if_different_return(x: u64, y: u64, cond: bool): u64 {
        if (cond) {
            x
        } else {
            y
        }
    }

    // One branch returns parameter, other computes - uses path-conditional ensures
    fun if_mixed_return(x: u64, cond: bool): u64 {
        if (cond) {
            x
        } else {
            x + 1
        }
    }

    // Chained comparison with short-circuit - uses path-conditional ensures
    fun test_chain_cmp(x: u64, y: u64, z: u64): bool {
        x < y && y < z
    }
}
