module 0x99::FunctionCall {
    use std::vector;
    use std::bit_vector;

    fun foo(x: u64): u64 {
        x + 1
    }

    fun foo_ref(x: &u64): u64 {
        *x + 1
    }

    fun foo_vec(): vector<u8> {
        vector::empty<u8>()
    }

    fun foo_grand_child(x: &mut u64): u64 {
        *x + 1
    }

    fun foo_child(x: u64): u64 {
        let ref = &mut x;
        foo_grand_child(ref)
    }

    fun foo_with_external_call(x: u64): u64 {
        // simulate an external call by calling a native function
        let bv = bit_vector::new(x);
        bit_vector::length(&bv)
    }


    // `foo(y)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of the second call to `foo` needs to be adjust on stack (one st_loc and one move_loc)
    fun bar(y: u64): u64 {
        foo(y) + foo(y)
    }

    // `foo(y)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `u64` copied once for reuse
    fun bar_optimal(y: u64): u64 {
        let temp = foo(y);
        temp + foo(y) + temp
    }

    // `foo_ref(ref)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `&u64` flushed and copied twice
    // - the result of the second call to `foo` needs to be adjust on stack (one st_loc and one move_loc)
    fun bar_ref(y: u64): u64 {
        let ref = &y;
        foo_ref(ref) + foo_ref(ref)
    }

    // `foo_ref(ref1)` can be reused
    //  even when there is a mutable reference `mut_ref` in between
    //  (here, `mut_ref` does not change the value of `y`)
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `&u64` flushed and copied twice
    // - the result of the second call to `foo` needs to be adjust on stack (one st_loc and one move_loc)
    fun bar_ref2(y: u64): u64 {
        let ref1 = &y;
        let v1 = foo_ref(ref1);
        let mut_ref = &mut y;
        let ref2 = &y;
        let v2 = foo_ref(ref2);
        v1 + v2
    }

    // `foo_vec()` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost: none
    fun bar_vec(): vector<u8> {
        foo_vec();
        foo_vec()
    }

    // `foo_child(y)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of the second call to `foo_child` needs to be adjust on stack (one st_loc and one move_loc)
    fun bar_recursive(y: u64): u64 {
        foo_child(y) + foo_child(y)
    }

    // `foo_with_external_call(y)` can be reused
    // perf_gain: 1 function call eliminated
    // new_cost:
    // - `u64` flushed and copied twice
    // - the result of the second call to `foo_with_external_call` needs to be adjust on stack (one st_loc and one move_loc)
    fun bar_with_external_call(y: u64): u64 {
        foo_with_external_call(y) +
        foo_with_external_call(y)
    }
}
