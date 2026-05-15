// `result_of<f>` is rejected for `f` returning no values. For the
// post-state of a `&mut` argument, the user must write `ensures_of<f>(...)`
// instead.
//
// `result_of<f>` is also rejected when the user supplies an explicit
// `&mut` post-state slot: the `&mut` parameter appears once and the prover
// supplies the pre/post split automatically.
module 0x42::result_of_void_invalid {

    fun apply_void(f: |u64|, x: u64) { f(x) }
    spec apply_void {
        // error: `result_of` cannot be used with functions that have no
        // return value.
        ensures result_of<f>(x) == 0;
    }

    fun apply_void_mut(f: |&mut u64|, x: &mut u64) { f(x) }
    spec apply_void_mut {
        // error: `result_of` cannot be used with functions that have no
        // return value.
        ensures result_of<f>(x) == 0;
    }

    fun apply_mut(f: |&mut u64| u64, x: &mut u64): u64 { f(x) }
    spec apply_mut {
        // error: `result_of<f>` takes only `f`'s input parameters; the `&mut`
        // post-state slot is not user-facing.
        ensures result == result_of<f>(old(x), x);
    }
}
