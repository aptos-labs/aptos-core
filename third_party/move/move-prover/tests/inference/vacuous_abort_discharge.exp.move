// Weakest-precondition inference is exact for loop-free code, so every abort
// obligation it emits is satisfiable. Two kinds are discharged here: type
// bounds (`u64` operands cannot approach `MAX_U128`) and the path condition
// (`x >= y` refutes `x - y < 0`).
//
// flag: --inference
// flag: -T=20
module 0x42::vacuous_abort_discharge {

    // The guard makes both subtractions non-negative, and `lo < hi` follows by
    // chaining `lo <= x` with `x < hi`, so the division cannot be by zero.
    // Since `delta < span`, the quotient is below 100 and the function is total.
    fun interpolate(lo: u64, hi: u64, x: u64): u64 {
        if (x < lo) { return 0 };
        if (x >= hi) { return 1 };
        let span = ((hi - lo) as u128);
        let delta = ((x - lo) as u128);
        ((delta * 100 / span) as u64)
    }
    spec interpolate(lo: u64, hi: u64, x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == (if (x < lo) 0 else if (x >= hi) 1 else (((x - lo) as u128) * 100 / ((hi - lo) as u128)) as u64);
        aborts_if [inferred] false;
    }


    // General affine interpolation has one real abort: an inverted value range.
    // Once `high_value - low_value` succeeds, `delta < span` bounds the bump by
    // that range, so both its u64 cast and `low_value + bump` are safe.
    fun interpolate_between(
        low_lock: u64,
        low_value: u64,
        high_lock: u64,
        high_value: u64,
        x: u64
    ): u64 {
        if (x < low_lock) { return 0 };
        if (x >= high_lock) { return high_value };
        let span = ((high_lock - low_lock) as u128);
        let range = ((high_value - low_value) as u128);
        let delta = ((x - low_lock) as u128);
        let bump = ((range * delta / span) as u64);
        low_value + bump
    }
    spec interpolate_between(low_lock: u64, low_value: u64, high_lock: u64, high_value: u64, x: u64): u64 {
        pragma opaque = true;
        ensures [inferred] result == (if (x < low_lock) 0 else if (x >= high_lock) high_value else low_value + ((((high_value - low_value) as u128) * ((x - low_lock) as u128) / ((high_lock - low_lock) as u128)) as u64));
        aborts_if [inferred] x >= low_lock && x < high_lock && high_value < low_value;
    }


    // Sums of `u8` widened to `u16` cannot overflow `u16`: the maximum is 765.
    fun widen_sum(a: u8, b: u8, c: u8): u16 {
        (a as u16) + (b as u16) + (c as u16)
    }
    spec widen_sum(a: u8, b: u8, c: u8): u16 {
        pragma opaque = true;
        ensures [inferred] result == (a as u16) + (b as u16) + (c as u16);
        aborts_if [inferred] false;
    }


}
/*
Verification: Succeeded.
*/
