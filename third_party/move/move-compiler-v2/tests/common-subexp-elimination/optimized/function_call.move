module 0x99::FunctionCall {
    use std::vector;
    use std::bit_vector;

    fun foo(x: u64): u64 {
        x + 1
    }

    fun foo_vec(): vector<u8> {
        vector::empty<u8>()
    }

    // `foo(y)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - None
    fun bar_optimal(y: u64): u64 {
        let temp = foo(y);
        temp + foo(y) + temp
    }

    // `foo_vec()` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost: none
    fun bar_vec(): vector<u8> {
        foo_vec();
        foo_vec()
    }
}
