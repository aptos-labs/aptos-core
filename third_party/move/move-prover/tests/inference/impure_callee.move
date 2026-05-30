// Test that try_as_pure_spec_call correctly rejects impure callees.
//
// count_up passes the first three checks in try_as_pure_spec_call:
// 1. No &mut params and no function-type params
// 2. Has an associated spec function with a body (derived from the Move function)
// 3. Empty used_memory (no global resource access)
//
// However, the fourth check (FunctionPurenessChecker in Specification mode) detects
// that the while loop body contains Assign operations, which are not allowed in spec
// expressions. So try_as_pure_spec_call returns None and the caller falls back to
// behavior predicates (result_of<count_up>(n)) instead of calling count_up(n) directly.
module 0x42::impure_callee {
    // A function with a while loop: contains Assign operations → impure under spec
    // semantics. The fourth check in try_as_pure_spec_call rejects it, so callers
    // use result_of<count_up>(n) rather than count_up(n) directly.
    fun count_up(n: u64): u64 {
        let i = 0;
        let s = 0;
        while (i < n) {
            s = s + 1;
            i = i + 1;
        } spec {
            invariant s == i;
            invariant i <= n;
        };
        s
    }

    // Caller: inference falls back to result_of<count_up>(n) because count_up
    // is impure under spec semantics (while loop with Assign operations).
    fun double_count(n: u64): u64 {
        count_up(n) * 2
    }
}
