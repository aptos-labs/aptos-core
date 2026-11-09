// A call through a function value whose transparent target has a `&mut`
// parameter does not return the target's `result_of` carrier, so a clause
// equating the result with it is marked solver-hard.
// flag: --verify-only=closure_carrier_mut_param::call
module 0x42::closure_carrier_mut_param {
    fun g(r: &mut u64, x: u64): u64 {
        *r = x;
        x
    }
    spec g(r: &mut u64, x: u64): u64 {
        ensures result == x;
        ensures r == x;
        aborts_if false;
    }

    fun call(r: &mut u64, x: u64): u64 {
        let f: |&mut u64, u64| u64 has drop = g;
        f(r, x)
    }
    spec call(r: &mut u64, x: u64): u64 {
        pragma opaque = true;
        ensures [inferred = sathard] result == result_of<g>(old(r), x);
        ensures [inferred] ensures_of<g>(old(r), x, result, r);
        aborts_if [inferred] false;
    }

}
/*
Verification: Succeeded.
*/
